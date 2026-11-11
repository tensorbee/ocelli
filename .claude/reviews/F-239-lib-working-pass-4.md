# F-239 review, pass 4, library half of the viewer part

**Reviewed**: `work/f-239-lib`, range `a166f718..6163b82f` (one commit, `6163b82f5002fa011e1f0a7fc67b4ce278eff851`, "F-239, library review pass 3, the one palette rule stated everywhere"), worktree `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-239-lib`, HEAD `6163b82f`, clean apart from untracked `.venv` and the `tools/oracle/out` symlink. The class sweep read every `.md` and `.rs` file in `git diff --name-only 05d9aca2..HEAD`, excluding `.claude/reviews`, as whole files and not only their added lines. No unstaged paths were reviewed.
**Author / reviewer**: range written by a Claude implementer remediating `.claude/reviews/F-239-lib-working-pass-3.md`. Reviewed by an independent Claude reviewer under `/microscope F-239 --working`, pass 4. The reviewer wrote none of this range and none of passes 1 to 3.
**Result**: 1 defect, 2 smells, 2 nitpicks.

F-239-lib pass 4: 1 defect, 2 smells, 2 nitpicks.

The code is right. The one palette rule is implemented exactly, and every site pass 3 listed now states it. What remains is the same prose class for a fourth pass, in four sentences no earlier grep matched, three of them added in this half, plus two plan table gaps the range itself opened. The class is shrinking and the locations keep moving, which the operator should know.

## Acceptance challenged

- **The rule, derived by hand before running the code.** A probe in the scratch copy called `PaletteColorLut::with_entry_bits` and printed the flag and `channel_entries`. Expected values come from the rule and from `x >> (n - 8)`, not from the probe.

| Case | Declared bits | Entries | Expected by the rule | Printed |
|------|---------------|---------|----------------------|---------|
| all 8-bit | `8 8 8` | `0 100 255` each | no wide channel, so not read as 8-bit, flag clear, identity, `0 100 255` x3 | flag false, `0 100 255` x3 |
| 12-bit, all small | `12 12 12` | `0 100 255` each | every wide channel fits, flag set, `0 100 255` x3 | flag true, `0 100 255` x3 |
| mixed 16-bit | `16 16 16` | `0 100 255`, `0 100 255`, `0 100 256` | blue holds 256, flag clear, every channel `>> 8`: `0 0 0 / 0 0 0 / 0 0 1` | flag false, `0 0 0 0 0 0 0 0 1` |
| conforming 16-bit | `16 16 16` | `0 32768 65535`, `65535 256 255`, `0 100 255` | flag clear, `0 128 255 / 255 1 0 / 0 0 0` | flag false, `0 128 255 255 1 0 0 0 0` |
| 9-bit, all small | `9 9 9` | `0 100 255` each | flag set, identity | flag true, `0 100 255` x3 |
| narrow red, small wide | `8 16 16` | `200 1 2`, `0 100 255`, `0 100 255` | red takes no part, both wide fit, flag set, identity | flag true, `200 1 2 0 100 255 0 100 255` |
| 12-bit, one large | `12 12 12` | `0 100 255`, `0 100 255`, `0 256 4095` | flag clear, `>> 4`: `0 6 15 / 0 6 15 / 0 16 255` | flag false, `0 6 15 0 6 15 0 16 255` |

  The code at `crates/ocelli-pixel/src/color.rs:179` to `:189` is the rule: `any_wide` over `bits > 8`, then `all` over the entries of the channels with `bits > 8` against `255.0`, then the identity for all three or each channel's own `DisplayScale`. A channel at 8 bits is excluded from both halves, and the all-8-bit case is false through `any_wide`.
- **Pass 3 replayed.**
  - D1, the plan. R1 at `:571` now requires at least one wide channel and states that an all-8-bit palette is not flagged. AC22 at `:404` says three 16-bit descriptors. The floor row at `:494` is scoped to a wide channel holding an entry above 255. Closed, though the scoping opened S1 below.
  - D2. `frame-open.md:133` to `:135` and `:147` to `:150`, `pixel-pipeline.md:474` to `:500`, `colour.rs:131` to `:134` and the `scales` field doc at `color.rs:107` each state the rule or defer to it. Each holds against the probe. Closed.
  - D3. All seven test comments pass 3 named now state the condition or the lookup. Closed. Their twins are D1 below.
  - S1. `a_twelve_bit_palette_of_8_bit_entries_is_read_as_8_bit` exists and asserts the flag and `0 100 255` three times, values that follow from the rule. The mutation is red on it, below. Closed.
  - N1. `pixel-pipeline.md` is reflowed. N2. The test is renamed and AC22 cites the new name.
- **Wasm reach, executed rather than inherited.** The range changes no non-comment line in any `src` file, but it moves line numbers in `color.rs`, which can move panic locations in a wasm module. Both modules were rebuilt from the scratch copy of `6163b82f`: render 2,179,981 bytes, sha1 `220ba1b4`, decode 16,455 bytes, sha1 `90713638`. Both equal pass 3's tip hashes, so `build-targets.md:294` to `:307` still holds as written. `cargo tree` reaches `ocelli-pixel` and `ocelli-render` and no `ocelli-codec` or `ocelli-dicom` by default, and only `ocelli-cache`, `ocelli-core` and `ocelli-pixel` among workspace crates under `decode-worker`. `crates/ocelli-wasm/src` names none of `colour_bytes_into`, `palette_color_lut` or `entries_read_as_8_bit`.
- **Ties.** `byte_from_channel` at `cpu.rs:1161` compares against the exact half and sends a tie to the even code, and its doc's claim about monochrome reaching it only with whole table bytes matches its callers. `display_index` at `colour.rs:485` adds `0.5` and its doc says unchanged by F-239, which is ruling R2. No tie sentence in the swept files was shown false. See N2 for the operator's tie record.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-pixel -p ocelli-render -p ocelli-dicom` | worktree at 6163b82f | now | exit 0. 52 result lines, 557 passed, 0 failed, 83 ignored. The new 12-bit test ran and passed |
| Probe `zz_probe.rs`, scratch only, deleted before the mutation | `git archive` of 6163b82f in `scratchpad/f239lib-p4rev/tree`, `CARGO_TARGET_DIR` `scratchpad/f239lib-p4rev/target` | now | the table above |
| S1 mutation: both `*bits > 8` at `color.rs:180` and `:184` changed to `*bits == 16`, then `cargo test --no-fail-fast` over the three crates | the same scratch copy and target | now | 556 passed, 1 failed, and the one failure is `a_twelve_bit_palette_of_8_bit_entries_is_read_as_8_bit` at `palette.rs:517`, `assertion failed: palette.entries_read_as_8_bit()`. Restored from a saved copy and diffed equal to `git show 6163b82f:crates/ocelli-pixel/src/color.rs` |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p ocelli-render --no-deps` | scratch copies of 6163b82f and of 05d9aca2 | now | both fail with the same four errors: three at `interpolation.rs:92:23`, `:92:41` and `:95:54` (`blend_display` links to private `texel_of`) and one at `graph.rs:1856:11` (`register_stored` links to private `selected_stored_format`). `git diff --stat 05d9aca2..HEAD` on those two files is empty. They predate this half and are not counted |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p ocelli-pixel -p ocelli-dicom --no-deps` | scratch copy of 6163b82f | now | exit 0 |
| `wasm-pack build crates/ocelli-wasm --target web`, default and `--no-default-features --features decode-worker` | scratch copy of 6163b82f, `target-w` | now | exit 0 both. Sizes and hashes above |
| `cargo tree` for both feature sets, `grep` of `crates/ocelli-wasm/src` | worktree at 6163b82f | now | as above |
| `cargo fmt --check` for the three crates | worktree | now | exit 0 |
| `git diff -U0 a166f718..6163b82f` filtered to non-comment `src` lines | the range | now | none |
| `git diff -U0 a166f718..6163b82f` added lines for U+2014, U+2013 and prose `;` | the range | now | none |
| `ocelli-compare produce`, 102 views, 0 bytes differ | pass 2 at 72066de0 | prior receipt | not rerun, the range changes no pixel |

## Defects

### D1, four sentences still give a channel's shift as following from its declared depth alone

**Where**: `.claude/plans/F-239-design.md:329`, `crates/ocelli-render/tests/cpu.rs:1019` to `:1021`, `crates/ocelli-render/tests/colour_bytes.rs:220`, `crates/ocelli-pixel/tests/palette.rs:172` to `:173`.
**What**:
- Plan `:329`, the provider table's Display scaling row: the policy is "Bits Stored for samples, each palette descriptor's third value for entries". That is the rule before ruling R1, stated as the provider's policy with no exception. The file is in the sweep, and the line is unchanged since 05d9aca2, which is why pass 3's added-line sweep did not reach it.
- `cpu.rs:1020`: "Its descriptor's third value is 16, so the entry is shifted right by 8". Pass 3's D3 flagged `cpu.rs:984` in this same test for giving the descriptor as the reason, and the range fixed `:984` and not this paragraph forty lines below it.
- `colour_bytes.rs:220`: "AC22's bytes. Descriptors `5\0\16`, so every channel shifts by 8".
- `palette.rs:172`: "The entries are 16-bit, so F-239's display scale shifts each one right by 8".
**Why it is wrong**: a 16-bit descriptor does not make a channel shift. The probe's 16-bit and 12-bit all-small palettes are not shifted. Each test's shift actually follows from a wide channel holding an entry above 255 (1000, 256 and up, and 40000 and up), which none of the three sentences names. This is the shape the range corrected at `frame_open.rs:1055` and at `palette.rs:304`, where the new text says "holds an entry above 255, so the palette is not read as 8-bit". Microscope item 3, and HLD 27.2 R2's point that a test's prose is read as its specification. A reader taking `:329` as the provider's contract would expect the corpus palettes, both 16-bit with entries of at most 255, to draw black.
**Evidence**: the probe's `16 16 16` and `12 12 12` all-small rows. `git diff 05d9aca2..HEAD` shows the three test sentences as added in this half.
**Scope and regression**: present on HEAD, prose only. Smallest fix: `:329` says "Bits Stored for samples, the one palette rule of ruling R1 for entries". Each test sentence names its cause, for example "Red holds 1000, above 255, so the palette is not read as 8-bit and the entry is shifted right by 8". `grep -nE ', so (every|each|the) (channel|entry|one)'` over the swept files lists `colour_bytes.rs:220` and `cpu.rs:1020` beside `palette.rs:310` and `:375`, which are true because the doc above each names the condition, and `plan:212` and `tests/cpu.rs:173`, which are not about the palette rule. It misses `:329` and `palette.rs:172`, so no single grep replaces reading the sentence.

## Smells

### S1, the floor table no longer has a row for an all-8-bit palette

**Where**: `.claude/plans/F-239-design.md:494` and `:495`.
**What**: before this range `:494` was "PALETTE COLOR, 8 and 16 bits", which covered the common 8-bit palette. The range scoped it, as pass 3 suggested, to "a channel declared above 8 bits holding an entry above 255", and `:495` requires at least one channel declared above 8 bits. A palette whose three descriptors all declare 8 bits now matches neither row.
**Failure mechanism**: the section's opening sentence at `:484` says every behaviour cornerstone3D 5.8.2 has for these images Ocelli has too, verified or named as a gap. The 8-bit palette is the case most files carry and the table is where a reviewer or the viewer half checks the floor, so the most common palette is the one with no stated outcome. The outcome is Same, identity on both sides, by the probe's all-8-bit row. Pass 3's suggested wording caused this, so the gap is the reviewer's as much as the implementer's.
**Smallest regression**: one row, "PALETTE COLOR, every channel declared 8 bits | Drawn, entries as they are | Drawn, identity, not flagged | Same. `an_all_8_bit_palette_does_not_report_the_fallback`, AC27's 8-bit row if any".

### S2, AC22 claims the 12-bit fallback and does not name the test that holds it

**Where**: `.claude/plans/F-239-design.md:404`, the Test / observation column.
**What**: the row now says "Three 12-bit descriptors with the same entries do the same", and still names only `palette_entries_are_shifted_by_their_descriptor` and `a_palette_is_read_as_8_bit_only_when_every_wide_channel_fits_in_8_bits`. Under the exactly-16-bits mutation both named tests stay green, and the only red test is `a_twelve_bit_palette_of_8_bit_entries_is_read_as_8_bit`, which the row does not name. The row's "12-bit channel ... shifts by 4" and "Bits 17 refuse by name" are held by `each_palette_channel_is_scaled_by_its_own_depth`, also unnamed, which is older than this range.
**Failure mechanism**: AC22's column is how a later story or reviewer finds what guards the row. Someone pruning tests against AC22 would keep the two named tests and could delete the one test that makes pass 3's S1 boundary observable, and the suite would stay green. That is pass 3's S1 coming back through traceability.
**Smallest regression**: add `a_twelve_bit_palette_of_8_bit_entries_is_read_as_8_bit` and `each_palette_channel_is_scaled_by_its_own_depth` to the column.

## Nitpicks

- **N1.** `crates/ocelli-pixel/tests/palette.rs:443` is 105 characters in a file whose comments wrap near 80, and `crates/ocelli-dicom/tests/frame_open.rs:1247` ends at "the page states. This is the", an edit not reflowed. `cargo fmt --check` passes because this repository's `rustfmt.toml` does not wrap comments.
- **N2.** The operator's tie record at plan `:558` to `:565` still says "for monochrome and colour alike" and "Monochrome moves too". Ruling R2 at `:572` names that section and supersedes it, so it reads as the record of the answer as given, as item 8 does for R1. The range gave item 8 a forward pointer to R1 and the tie record has none to R2. A one-line pointer would let a reader of that section alone find the ruling. Not counted.

## Verified clean

- **The code against the rule**, by hand-derived bytes for seven palettes including all four the brief named, and by the S1 mutation, which is red on exactly the new test and nothing else across 557 tests.
- **Every sentence the range changed**, read against the probe: plan items 8 and 11, AC22, the Tier A boundary line, floor rows `:494` and `:495`, R1. `color.rs:107`, `:125`, `:143` to `:152`, `:230` to `:236`, `:287` to `:291`. `colour.rs:131` to `:134`, which names the lookup tier C reads, true because `bake` maps through the resolved transform and tier C maps through `ColorTransform::map_into` at `cpu.rs:957`. The six changed test doc comments. `frame-open.md:133` to `:135` and `:147` to `:150`. `pixel-pipeline.md:464` to `:500`, including "Before F-239" against pass 3's reading of `05d9aca2:cpu.rs:120` and `colour.rs:131`, and "a palette whose wide channels hold only entries of at most 255 draws as before", which also holds vacuously for an all-8-bit palette.
- **The independent sweep.** Whole-file greps over the 22 swept files for `own descriptor`, `never from the data`, `from the header`, `table declared`, `table whose`, `per (table|channel)`, `by its descriptor`, `declared above 8`, `fits? in 8`, `at most 255`, `above 255`, `read as 8-bit`, `fallback`, `third value`, `shifted right by`, `shifts? by`, `scaled (by|as|across)`, `display scale`, `depth of`, `>> ?8`, then for ties `tie`, `even`, `half`, `round`, and for reach `wasm`, `decode-worker`, `byte-identical`, `reach`. About 190 hits were read in context. The false ones are D1 and nothing else. True and checked include `palette.rs:191`'s "third value ... is what decides", which is about the entry container a reader must not infer from the data and is consistent with R1, since R1 never refuses or truncates an entry above 255, `frame_open.rs` test `:1211`'s `40000 >> 8 = 156`, `lut.rs:127` and `:420` and `pixel-pipeline.md:200`, which are the VOI LUT range and not the palette scale, and `pixel-pipeline.md:569`'s guard sentence. The tie hits that say ties up, `tests/cpu.rs:5`, `:173` and `:210`, `colour.rs:471` and `render-graph.md:427`, all describe the monochrome table address that ruling R2 keeps.
- **Boundary.** No `as`, `#[allow]`, `unsafe` or `wasm-bindgen` added in the range, no non-comment `src` line changed, no em-dash, en-dash or prose semicolon added. `README.md`, `use.html` and `more.html` are untouched in `05d9aca2..HEAD`.
- **The implementer's rustdoc report.** Four pre-existing private intra-doc link errors in `ocelli-render`, identical at 05d9aca2 and at the tip, in files this half does not touch. The implementer's location list gave `graph.rs:1856` and `interpolation.rs:92`, and the fourth error sits at `interpolation.rs:95`. Not counted.

## Limitations and handoff

- `build-targets.md`'s 504-byte `twiggy diff` attribution was not re-run. The module sizes and hashes around it were.
- The corpus producer comparison was not rerun, because the range changes no pixel. Pass 2's receipt at 72066de0 stands.
- No GPU test was run. The bake fixtures read CPU-baked bytes and `byte_ties.rs`'s ignored tests were not exercised.
- The viewer half, the oracle's reader replacement and the page's use of the flag were not reviewed.
- Next owner: the implementer for D1, the lead or the implementer at the lead's direction for S1 and S2, which are plan text. Then a fresh independent pass. This pass has blocking findings, so the library half does not pass.
- Scratch `scratchpad/f239lib-p4rev` is removed with Python `shutil` after this file is written.
