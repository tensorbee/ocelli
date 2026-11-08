# F-239 review, pass 3, library half of the viewer part

**Reviewed**: `work/f-239-lib`, range `40b0ef8b..ee3c7a2a` (one commit, `ee3c7a2a02937f06f5795613c3aa5ac0a0013a9a`, "F-239, library review pass 2 fixes"), worktree `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-239-lib`, HEAD `ee3c7a2a`, clean apart from untracked `.venv` and the `tools/oracle/out` symlink. The class sweep covered `git diff 05d9aca2..HEAD` over `.md` and `.rs`, excluding the two earlier review files, which are records of earlier trees. No unstaged paths were reviewed.
**Author / reviewer**: range written by a Claude implementer remediating `.claude/reviews/F-239-lib-working-pass-2.md`. Reviewed by an independent Claude reviewer under `/microscope F-239 --working`, pass 3. The reviewer wrote none of this range and wrote neither pass 1 nor pass 2.
**Result**: 3 defects, 1 smell, 2 nitpicks.

F-239-lib pass 3: 3 defects, 1 smell, 2 nitpicks.

The class pass 2 named, a sentence saying a palette's depth comes from each channel's own descriptor or from the header alone, survives in places pass 2 did not list. Its fix touched the sites pass 2 named and not their twins in `docs/lld/frame-open.md`, `ocelli-render` and the test doc comments. That is the same finding a third pass running, in new locations, and is worth the operator knowing. The list below is meant to be the whole of what remains, found by sweeping every added sentence in the half rather than by following pass 2's list.

## Acceptance challenged

- **Pass 2 replayed through the real code.**
  - D1, the plan. R1 at `:571` now says "the whole palette". Item 11 at `:232` names the fallback as the one data-dependent exception. AC27 and the consumer row at `:343` expect both corpus rows unchanged. AC22 names both tests. A floor row for the fallback case was added at `:495`. Checked against the corpus: `pydicom` on the main checkout's `corpus/data/synthetic/sc_palette_color.dcm` gives descriptors `256\10\16` on all three channels, and `sc_palette_color_16.dcm` gives `0\0\16`, every channel's largest entry 255, so ruling R1 reads both as 8-bit, as AC27 now says. What is still false is in D1 below.
  - D2, doc comments and LLD. Every site pass 2 listed is corrected and each corrected sentence holds against the probe below. The twins pass 2 did not list are D2 and D3 below.
  - S1. `an_all_8_bit_palette_does_not_report_the_fallback` exists, and M15, pass 2's M4 (`any_wide` forced true), is now red on exactly that test.
  - S2. `disagreeing_descriptors_name_the_first_that_differs_from_red` exists. Its four cases, hand-checked against PS3.3 C.7.6.3.1.5's three descriptor values: green count 2 against red's 3 names `(0028,1102)`, blue first mapped 5 names `(0028,1103)`, both differing names green, and green's first mapped value differing with blue's count names green. M16 to M20 below, every wrong culprit and each half of the comparison, are red.
  - N1 and N2. The repeated sentence in `build-targets.md` is gone, and `frame-open.md`'s R1 paragraph wraps at 78.
- **The fallback's depth range, derived here.** Ruling R1 applies to a channel "declared above 8 bits", and item 11's display scale accepts `9 ..= 16`. So a palette declared 12 bits on every channel with entries `0 100 255` falls back to the identity. A palette declared 12 bits whose blue holds 4095 is conforming, so `100 >> 4 = 6`, `255 >> 4 = 15`, `4095 >> 4 = 255`. A probe in the scratch copy printed flag true with entries `0 100 255` three times, then flag false with `0 6 15 / 0 6 15 / 0 6 255`. The code agrees. No fixture reaches either case, smell S1.
- **The flag's vacuous case.** An all-8-bit palette has no channel declared above 8 bits, so it takes no fallback and its flag is clear. The probe with `with_entry_bits` at `[8, 8, 8]` printed flag false, and S1's fixture asserts it. The plan's R1 sentence, read as written, says otherwise, D1.
- **What reaches the wasm build.** Executed now rather than trusted. `cargo tree -p ocelli-wasm -e normal --target wasm32-unknown-unknown --prefix none` reaches `ocelli-pixel` and `ocelli-render` and no `ocelli-codec`, `ocelli-dicom`, `openjph-core` or `ritk-codecs`. With `--no-default-features --features decode-worker` it reaches only `ocelli-cache`, `ocelli-core` and `ocelli-pixel` among workspace crates. `grep` finds no `colour_bytes_into`, `palette_color_lut` or `entries_read_as_8_bit` in `crates/ocelli-wasm/src`. The render module was rebuilt at 01bc1342 and at the tip, each in its own source tree and target directory: 2,179,981 bytes both, 8 bytes differ, so "not byte-identical to 01bc1342's but the same size" holds, and pass 2's unexecuted half of that sentence is now executed. The decode module was rebuilt at 699a5148 and at the tip: sha1 `90713638` on both, 16,455 bytes, byte-identical as `build-targets.md` says.
- **Ties.** `byte_from_channel` at `crates/ocelli-render/src/cpu.rs:1161` compares with the exact half and sends an exact tie to the even code. Its callers in `cpu.rs` are `rgb_bytes`, reached by the colour draws and by `rgba_of_preset`, whose bytes are a preset table's whole codes, so the doc's "Tier C's monochrome draws reach this function only with a preset table's own bytes" holds on both the nearest and the linear monochrome path. `display_index` at `colour.rs:485` still adds 0.5 and its doc says ties up and unchanged by F-239, which matches R2. The grey ramp's `round_half_even` at `colour.rs:448` is what `render-graph.md:429`'s "already rounds to even" names. No tie sentence in the sweep was shown false.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-pixel -p ocelli-render -p ocelli-dicom` | worktree at ee3c7a2a | now | exit 0. 52 test binaries, 556 passed, 0 failed, 83 ignored. Both new tests ran and passed |
| Mutation catalogue M14 to M21, `scratchpad/f239lib-p3rev/tree`, its own `CARGO_TARGET_DIR`, each restored and asserted equal to the original text after | `git archive` of ee3c7a2a | now | 7 red, 1 green. Table under "Verified clean" and S1 |
| Probe test `zz_probe.rs`, scratch only | scratch copy of ee3c7a2a | now | 12-bit small: flag true, `0 100 255` x3. 12-bit with 4095 in blue: flag false, `0 6 15 / 0 6 15 / 0 6 255`. `[8, 16, 16]` with blue 256: flag false, `0 100 255 / 0 0 0 / 0 0 1`. `[8, 8, 8]`: flag false |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p ocelli-dicom -p ocelli-pixel --no-deps` | scratch copy of ee3c7a2a | now | exit 0, the new intra-doc link to `PaletteColorLut::with_entry_bits` resolves |
| `git diff -U0 40b0ef8b..ee3c7a2a -- 'crates/*/src/*.rs'` filtered to non-comment lines | the range | now | none, so the range changes no pixel and pass 2's producer measurement still describes this tree |
| `wasm-pack build crates/ocelli-wasm --target web --out-dir pkg`, at 01bc1342 and at ee3c7a2a | `git archive` trees, `target-w01` and `target-wtip` | now | exit 0 both. 2,179,981 bytes both, `cmp -l` counts 8 differing bytes. The tip's sha1 `220ba1b4` equals the worktree's own `pkg` |
| `wasm-pack build ... --out-dir pkg-decode -- --no-default-features --features decode-worker`, at 699a5148 and at ee3c7a2a | `git archive` trees, own target dirs | now | exit 0 both, sha1 `90713638` both, 16,455 bytes |
| `cargo tree` for both feature sets | worktree at ee3c7a2a | now | as stated above |
| `pydicom` on the two corpus palette rows | main checkout `corpus/data` | now | as stated above |
| `git show 05d9aca2:crates/ocelli-render/src/cpu.rs` and `colour.rs` | the story's approval commit | now | `:120` and `:131` refuse a palette entry outside `0..=255`, so `pixel-pipeline.md`'s "Before F-239" sentence holds |
| `ocelli-compare produce`, 102 views, 0 bytes differ | pass 2 at 72066de0 | prior receipt | not rerun, the range is comment and test only |

## Defects

### D1, the plan's R1 sentence sets the flag on an all-8-bit palette, and two plan sentences still state the rule per table

**Where**: `.claude/plans/F-239-design.md:571` (R1), `:404` (AC22), `:494` (floor table).
**What**:
- R1: "When every entry of every channel declared above 8 bits fits in 8 bits, the whole palette is read as 8-bit ... and the frame carries a flag the page states." For an all-8-bit palette no channel is declared above 8 bits, so the condition holds vacuously and the sentence says the flag is set. The code requires at least one wide channel (`color.rs:178`), the probe printed flag false at `[8, 8, 8]`, and pass 2's S1 fixture now asserts false. The viewer half reads the plan to decide when the page states the departure, so this is the sentence that would put the notice on every conforming 8-bit palette.
- AC22: "A 16-bit descriptor whose entries are `0 100 255` is read as 8-bit by ruling R1". The same row's next sentence gives two 16-bit descriptors with exactly those entries that are shifted to `0 0 0`, because blue holds 256. As a rule about one descriptor it is false, and the row contradicts itself.
- The floor row at `:494`, "PALETTE COLOR, 8 and 16 bits ... Drawn, every channel shifted by its own descriptor ... Better", covers every 16-bit palette, including the fallback palettes the new row at `:495` says are drawn unshifted and are the Same. Two rows give opposite outcomes for one palette.
**Why it is wrong**: false claims in prose, microscope item 3. Pass 1's ruling R4 says stale text is corrected in this change.
**Evidence**: the probe above, M15 red on `an_all_8_bit_palette_does_not_report_the_fallback`, and `a_wide_table_whose_entries_fit_in_8_bits_is_read_as_8_bit_and_flagged`'s `mixed` case, which asserts `0 0 0` for those two descriptors.
**Scope and regression**: present on HEAD, plan only, lead-owned under R4. Smallest fix: R1 gains "When at least one channel is declared above 8 bits and every entry of every such channel fits in 8 bits". AC22 says "Three 16-bit descriptors whose entries are all `0 100 255`". The `:494` row is scoped to "a wide channel holding an entry above 255".

### D2, library and LLD sentences still say a palette's scale comes from each channel's own descriptor, or state the fallback per table

**Where**: `docs/lld/frame-open.md:133` to `:136`, `docs/lld/pixel-pipeline.md:482` and `:497` to `:498`, `crates/ocelli-render/src/colour.rs:131` to `:134`, `crates/ocelli-pixel/src/color.rs:107`.
**What**:
- `frame-open.md`: "Each channel is built with `PaletteColorLut::with_entry_bits` and its declared depth, so the display scale of `docs/lld/pixel-pipeline.md` comes from that channel's own descriptor. A 12-bit descriptor is a 16-bit container shifted by 4." Under the fallback the scale is the identity whatever the descriptor says, and the probe's 12-bit palette with entries `0 100 255` is not shifted by 4. This is the LLD twin of the `frame_open.rs:409` doc comment that pass 2's D2 had corrected. The R1 paragraph eleven lines below then says the opposite.
- `pixel-pipeline.md:482`, in bold: "**A wide table whose entries all fit in 8 bits is read as 8-bit**". That is the per-channel rule, M1 in pass 2, and the same paragraph says the decision is once for the whole palette. `a_wide_table_whose_entries_fit_in_8_bits_is_read_as_8_bit_and_flagged`'s `mixed` case has two such tables shifted to `0 0 0`.
- `pixel-pipeline.md:498`: "a table of 8-bit entries draws as before". A 16-bit red table `0 100 255` beside a blue holding 256 was refused before F-239, because tier C and the bake refused any entry above 255 (`05d9aca2:crates/ocelli-render/src/cpu.rs:120`, `colour.rs:131`). Now it draws, shifted to `0 0 0`. Only a palette whose wide channels all fit in 8 bits draws as before.
- `colour.rs:132`, `ColourLut::palette`: "The entries arrive already scaled by their own descriptors through `PaletteColorLut`'s display scale". Under the fallback they arrive unscaled. The conclusion, that every texel is the value tier C draws, still holds, because both read `PaletteColorLut::lookup`.
- `color.rs:107`, the `scales` field: "Red, green and blue display scales, each from its own descriptor." Under the fallback all three are `DisplayScale::IDENTITY` (`color.rs:184` to `:188`). Pass 2's D2 corrected the `read_as_8_bit` field two lines below and not this one.
**Why it is wrong**: false sentences beside pixel arithmetic, HLD 27.3's review item, which tells a human to check the arithmetic against the specification and not against the comment. A reader trusting the `frame-open.md` sentence would expect a 12-bit palette of small entries to draw near black.
**Evidence**: the probe, and the `mixed` case's asserted `0 0 0 0 0 0 0 0 1`.
**Scope and regression**: present on HEAD, prose only, in the sweep base and not in this range. Smallest fix: the field doc says "each from its own descriptor, or the identity for all three under ruling R1". Elsewhere say "the palette's display scale" or "`PaletteColorLut`'s display scale" at each site and leave the rule to `with_entry_bits`, make the bold phrase "A palette whose wide channels all fit in 8 bits", and make `:498` "a palette whose wide channels hold only entries of at most 255 draws as before". Deleting the 12-bit sentence in `frame-open.md` is the smaller change.

### D3, seven test doc comments state the depth rule pass 2 removed from the library

**Where**: `crates/ocelli-pixel/tests/palette.rs:304` to `:306`, `:362` to `:364`, `:371` to `:373`, `crates/ocelli-dicom/tests/frame_open.rs:1244` to `:1245`, `crates/ocelli-render/tests/colour.rs:778` to `:779`, `crates/ocelli-render/tests/cpu.rs:984`, `crates/ocelli-pixel/tests/color.rs:697`.
**What**:
- `palette.rs:304`: "each channel is shifted right by `n - 8` by its OWN descriptor, decided from the header and never from the data." The same sentence as pass 2's D2 at `DisplayScale`, in the doc of `palette_entries_are_shifted_by_their_descriptor`.
- `palette.rs:362`: "A table declared above 8 bits whose every entry fits in 8 bits is read as 8-bit", and `:371`: "One entry of 256 makes the table a 16-bit table again, so it is shifted". Both state the per-channel rule, and the fixture under them asserts the opposite in its `mixed` case. The paragraph ends "The decision is once for the whole palette", so the doc contradicts itself.
- `frame_open.rs:1244`: "A table declared 16-bit whose entries all fit in 8 bits is read as 8-bit and the palette carries the flag". The per-table rule again, true of its fixture only because all three tables are identical.
- `tests/colour.rs:778`: "A 16-bit table whose entries all fit in 8 bits is read as 8-bit, so its texels are the entries themselves." Same, with three identical tables.
- `tests/cpu.rs:984`: "A palette entry is scaled by its own descriptor, so the GPU bake and this raster read the same display byte for it." True of the fixture's entry 1000, false as the general reason. The reason is that both read `PaletteColorLut::lookup`.
- `tests/color.rs:697`, the doc of `a_sample_scale_is_refused_on_a_palette_frame`: "Its entries are scaled by their own descriptors instead." Not under the fallback.
**Why it is wrong**: microscope item 3, and HLD 27.2 R2's point that a test's prose is read as its specification. A later implementer told by `palette.rs:362` what the fallback is would write M1, and the next line's fixture would then go red with a doc that agrees with the mutation.
**Evidence**: the `mixed` case's assertions at `palette.rs:404` to `:406`. Pass 2's M1, per-channel, red on that case.
**Scope and regression**: present on HEAD, test prose only. Smallest fix: state each as the palette rule, "a palette whose channels declared above 8 bits all fit in 8 bits", delete "decided from the header and never from the data" at `:306`, give `cpu.rs:984` the lookup as its reason, and make `tests/color.rs:697` "Its entries are scaled by `PaletteColorLut` instead".

## Smells

### S1, the fallback below 16 bits has no fixture

**Where**: `crates/ocelli-pixel/src/color.rs:178` and `:182`, the two `*bits > 8` tests.
**What**: M14 changes both to `*bits == 16`, so a channel declared 9 to 15 bits never joins the decision. The whole suite of `ocelli-pixel`, `ocelli-dicom` and `ocelli-render` stays green. Every fallback fixture declares 16 bits, and every 12-bit fixture (`each_palette_channel_is_scaled_by_its_own_depth`, `wide_palette_entries_are_scaled_by_their_descriptor`) holds an entry above 255.
**Failure mechanism**: with M14 a 12-bit palette whose entries all fit in 8 bits is shifted by 4, so `255` draws as `15`, nearly black, while R1, `with_entry_bits`'s doc and `pixel-pipeline.md` all say it is read as 8-bit and flagged. That is a quietly wrong palette with a green suite. The opposite edit, reading `> 8` as `>= 8`, is pass 2's M5 and is equivalent, so this boundary is the one that can move unobserved.
**Smallest regression**: one case in `a_wide_table_whose_entries_fit_in_8_bits_is_read_as_8_bit_and_flagged` or beside it, `with_entry_bits` at `[12, 12, 12]` with entries `0 100 255`, asserting the flag set and `0 100 255` three times. The probe's printed values are the expectation, and they follow from R1 and `4095 >> 4 = 255`, not from the probe.

## Nitpicks

- **N1.** `docs/lld/pixel-pipeline.md:465` is 37 characters and `:474` is 113, where the file wraps near 78. The edit was not reflowed, the shape of pass 2's N2.
- **N2.** The test name `a_wide_table_whose_entries_fit_in_8_bits_is_read_as_8_bit_and_flagged` states the per-table rule its own `mixed` case refutes. AC22 cites it by name, so a rename is the lead's call and does not block.

## Verified clean

- **S1 and S2 now guard what they claim.**

| ID | Mutation | Result |
|----|----------|--------|
| M14 | `any_wide` and the filter test `*bits == 16` in place of `> 8` | green, S1 |
| M15 | `any_wide` forced true, pass 2's M4 | red, `an_all_8_bit_palette_does_not_report_the_fallback`, the only failure across the whole `ocelli-pixel` suite |
| M16 | the struct's flag stored as `false` | red, `a_wide_table_whose_entries_fit_in_8_bits_is_read_as_8_bit_and_flagged` |
| M17 | culprit always green, pass 2's M7 | red, `disagreeing_descriptors_name_the_first_that_differs_from_red` |
| M18 | culprit always blue, pass 2's M8 | red, same test |
| M19 | `disagrees` drops the first mapped value | red, same test |
| M20 | `disagrees` drops the entry count | red, same test |
| M21 | culprit always red's `(0028,1101)` | red, same test |

- **Every sentence the range changed, executed.** Plan `:232`, `:343`, `:404`'s fallback rows and bytes (`255 >> 8 = 0`, `256 >> 8 = 1`), `:409` against the corpus, `:495`, and `:571` apart from D1's vacuous case. `color.rs`'s field doc, `new`, `with_entry_bits`, `entries_read_as_8_bit`, `channel_scale` and `DisplayScale` against the probe and against M15 and M16. `frame_open.rs:409`'s new doc against `with_entry_bits` and `cargo doc -D warnings`. `pixel-pipeline.md:464` to `:474` and `frame-open.md:149` to `:151`. `build-targets.md:300` to `:303` against the two rebuilds.
- **The sweep, every added sentence in `05d9aca2..HEAD` stating how a palette's or a sample's depth is decided or scaled, how ties round, or what reaches the wasm build.** About 450 candidate lines were read. The ones shown false are D1 to D3 and nothing else. True and checked: the sample scale from Bits Stored and its refusal on a palette (`color.rs`, `tests/color.rs`, `pixel-pipeline.md:464` to `:470`), the truncating shift (`255 >> 8 = 0`, `65535 >> 8 = 255`, `32768 >> 8 = 128`), `frame_open.rs`'s 8-bit-in-words reading and its refusal of a word above 255, the 65,536-entry case (`40000 >> 8 = 156`), `wide_palette_entries_are_scaled_by_their_descriptor`'s 16 and 12-bit rows, `pixel-pipeline.md`'s fallback body text after its bold phrase, its "Before F-239" sentence against `05d9aca2`, every tie sentence in `cpu.rs`, `colour.rs`, `byte_ties.rs`, `colour_bytes.rs`, `render-graph.md:413` to `:437` and the plan's item 10 and floor tie row, and every wasm sentence in `build-targets.md:293` to `:320` apart from the 504-byte attribution.
- **Boundary.** No `as`, `#[allow]`, `unsafe` or `wasm-bindgen` added in the range. The range changes no non-comment line in any `src` file. No em-dash, en-dash or prose semicolon in the range's added lines.
- `README.md`, `use.html` and `more.html` are untouched in `05d9aca2..HEAD`, and the flag still has no consumer outside tests.

## Limitations and handoff

- `build-targets.md`'s 504-byte `twiggy diff` attribution to named functions was not re-run. The sizes and byte identities around it were.
- The corpus producer comparison was not rerun, because the range changes no pixel. Pass 2's receipt at 72066de0 stands for the pixels.
- No GPU test was run. The bake fixtures read CPU-baked bytes and open no adapter, and `byte_ties.rs`'s ignored tests were not exercised.
- The viewer half, the oracle's reader replacement and the page's use of the flag were not reviewed.
- Outside the sweep: the plan's "Decided by the operator on 2026-11-07" item 8 at `:545`, unchanged since 05d9aca2, still says a palette's depth is "decided from the header". It is the record of the operator's answer as given, and ruling R1 at `:571` supersedes it. Not counted. The lead may want a pointer to R1 there.
- Next owner: the lead, or the implementer at the lead's direction, for D1. The implementer for D2, D3 and S1. Then a fresh independent pass. This pass found blocking findings, so the library half does not pass.
- The class in D2 and D3 has now survived three passes in moving locations. A grep the next pass can run in place of reading, `grep -rnE 'own descriptor|never from the data|A (wide|16-bit) table|table declared' crates/ocelli-pixel crates/ocelli-dicom crates/ocelli-render docs/lld .claude/plans/F-239-design.md`, lists every site above except `palette.rs:371` and `pixel-pipeline.md:498`, beside three hits that are true (`color.rs:124`, `color.rs:228` and plan item 11, each qualified by the fallback).
- Scratch `scratchpad/f239lib-p3rev` is removed with Python `shutil` after this file is written.
