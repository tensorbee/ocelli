# F-239 review, pass 5, library half of the viewer part

**Reviewed**: `work/f-239-lib`, range `HEAD~1..HEAD` (one commit, `0b90339a`, "F-239, library half pass 4 text fixes"), worktree `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-239-lib`, clean apart from untracked `.venv` and the `tools/oracle/out` symlink. The final palette sweep read every `.rs` and `.md` file in `git diff --name-only 05d9aca2..HEAD`, excluding `.claude/reviews`, including unchanged lines of the plan.
**Author / reviewer**: range written by the lead remediating `.claude/reviews/F-239-lib-working-pass-4.md`. Reviewed by an independent Claude reviewer under `/microscope F-239 --working`, pass 5. The reviewer wrote none of this range and none of passes 1 to 4.
**Result**: 1 defect, 0 smells, 3 nitpicks.

F-239-lib pass 5: 1 defect, 0 smells, 3 nitpicks.

Every sentence the commit changed is true against the code and the tests. Pass 4's D1, S1, S2 and N2 are closed. The final sweep found one more sentence of pass 4's D1 class, added in this half, in a test pass 4 did not list.

## Acceptance challenged

The rule, as `crates/ocelli-pixel/src/color.rs:179` to `:189` implements it: `any_wide` over `bits > 8`, then every entry of every channel with `bits > 8` at most `255.0`, then the identity for all three, else each channel's own declared scale.

- **Pass 4 D1, `cpu.rs:1019` to `:1022`.** `wide_descriptor` at `cpu.rs:1198` builds a 16-bit descriptor. Red is `0 1000`, green and blue `0 0`. All three are wide, red holds 1000, above 255, so the palette is not read as 8-bit and `1000 >> 8 = 3`. The test asserts `[3, 0, 0, 255]`. The sentence is true. Closed.
- **Pass 4 D1, `colour_bytes.rs:220` to `:221`.** Descriptors `5\0\16`, red holds 256, 32768 and 65535, green and blue 65535, 32768 and 256. Every channel is wide and holds an entry above 255, so every channel shifts by 8. The test asserts AC22's reply. True. Closed.
- **Pass 4 D1, `palette.rs:172` to `:174`.** Three 16-bit descriptors of `0 ..= 65535`, entries above 255 in every channel. `40000 >> 8 = 156` and `65535 >> 8 = 255`, which the test asserts. True. Closed.
- **Pass 4 D1, plan `:329`.** "each palette descriptor's third value for entries unless ruling R1 reads the whole palette as 8-bit". `PaletteColorLut::new` takes each descriptor's container, and `palette_color_lut` passes each declared third value through `with_entry_bits`. The exception is the rule. True. Closed.
- **Pass 4 S1, plan `:495`.** `an_all_8_bit_palette_does_not_report_the_fallback` exists at `palette.rs:496`. Its palette is `palette(FIRST_MAPPED)`, three descriptors declared 8 bits (`palette.rs:75` to `:81`), and it asserts the flag is clear. "Not flagged" is what it asserts. "Entries as they are" is held by the same palette's mapping tests at `palette.rs:102` and `:131`, which read the raw entries. The row's outcome, Same, follows from `any_wide` being false. Closed. See N2.
- **Pass 4 S2, AC22 at `:404`.** All four named tests exist, at `palette.rs:323`, `:379`, `:510` and `:447`. `palette_entries_are_shifted_by_their_descriptor` holds the 5 by 1 values and the clear flag. `a_palette_is_read_as_8_bit_only_when_every_wide_channel_fits_in_8_bits` holds the three 16-bit `0 100 255` channels flagged and read as they are, and the mixed palette shifted to `0 0 0 0 0 0 0 0 1` with the flag clear. `a_twelve_bit_palette_of_8_bit_entries_is_read_as_8_bit` holds "three 12-bit descriptors do the same". `each_palette_channel_is_scaled_by_its_own_depth` holds "a 12-bit channel in a palette not read as 8-bit shifts by 4", since green holds 51200 so the palette is not read as 8-bit and blue `3200 >> 4 = 200`, and "Bits 17 refuse by name" as `UnsupportedColourDepth`. Closed. See N3.
- **Pass 4 N2, plan `:561`.** "Ruling R2 below refines this record for monochrome." R2 at `:573` keeps monochrome's `display_index` rounding and corrects the tie section. True.
- **Commit hygiene.** No non-comment `.rs` line changed. No em-dash, en-dash or prose semicolon in the added lines.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-pixel -p ocelli-render -p ocelli-dicom` | worktree at 0b90339a | now | exit 0. 52 result lines, 557 passed, 0 failed, 83 ignored |
| `git diff -U0 HEAD~1..HEAD` over `crates` and the plan, added lines for U+2014, U+2013 and the semicolon | the range | now | none |
| `git diff -U0 HEAD~1..HEAD -- '*.rs'` filtered to non-comment lines | the range | now | none |
| `grep` over the 22 swept files for palette, entry or descriptor lines also matching shift, `>> 8`, read as 8, 8-bit, third value, declared, scale, fallback, above 255, at most 255, fits or depth, each hit read in context | worktree at 0b90339a | now | about 150 hits. D1 is the one contradiction |
| `git diff 05d9aca2..HEAD -- crates/ocelli-pixel/tests/palette.rs` | the half | now | D1's sentence is an added line |

## Defects

### D1, one more sentence gives a palette shift as following from the 16-bit descriptor alone

**Where**: `crates/ocelli-pixel/tests/palette.rs:223` to `:224`, in `bits_per_entry_comes_from_the_descriptor_and_not_from_the_data`.
**What**: "Stored 255 reads entry 255, which is `255 * 257 = 65535`, and the 16-bit descriptor shifts it right by 8 to 255."
**Why it is wrong**: under ruling R1 a 16-bit descriptor does not shift an entry. `a_palette_is_read_as_8_bit_only_when_every_wide_channel_fits_in_8_bits` at `palette.rs:379` shows three 16-bit descriptors read unshifted. The shift here follows from the wide channels holding entries above 255 (every entry from index 1 up is at least 257), which the sentence does not name. It is the shape pass 4's D1 corrected at `palette.rs:172`, fifty lines above, and at `cpu.rs:1020`, in a test pass 4 did not list. The test's own doc at `:192` says the third value "is what decides", which is about the entry container and holds, so the comment under it reads as a second statement that the descriptor decides the shift.
**Evidence**: `git diff 05d9aca2..HEAD -- crates/ocelli-pixel/tests/palette.rs` shows `:224` as an added line in this half. The asserted value is right, only the stated cause is wrong.
**Scope and regression**: present on HEAD, prose only. Smallest fix: "Stored 255 reads entry 255, which is `255 * 257 = 65535`. Every channel holds entries above 255, so ruling R1 does not read the palette as 8-bit and the entry is shifted right by 8 to 255." The comment at `:228`, "Entry 1 is 257, and 257 >> 8 = 1", then follows.

## Smells

None.

## Nitpicks

- **N1.** `crates/ocelli-dicom/tests/frame_open.rs:1209` to `:1210` says "through the 16-bit display scale stored 40000 is `40000 >> 8 = 156`". The table is `0 ..= 65535` in all three channels, so the shift holds, and "the 16-bit display scale" names the scale applied rather than giving the descriptor as its cause. Pass 4 read it as true. It is D1's nearest neighbour, and naming the entries above 255 would make it immune to the same reading. Not counted.
- **N2.** Plan `:495` cites only `an_all_8_bit_palette_does_not_report_the_fallback`, which asserts the flag and no entry. "Drawn, entries as they are" is held by `the_first_mapped_stored_value_offsets_the_whole_table` and `ignoring_the_offset_shifts_the_whole_colour_map`, which map the same 8-bit `palette(FIRST_MAPPED)` at `palette.rs:101` and `:131` and which the row does not name. Not counted.
- **N3.** AC22's "Reply `0 255 255 0 128 128 1 1 1 128 0 0 255 0 0`" is a byte claim. `palette_entries_are_shifted_by_their_descriptor` asserts the same values as `f32` stage output, and the bytes themselves are asserted by `crates/ocelli-render/tests/colour_bytes.rs:231`, `sixteen_bit_palette_entries_are_shifted_before_their_byte`, which the column does not name. The `f32` values are whole numbers, so the byte follows, and the omission predates this commit. Not counted.

## Verified clean

- Every sentence the commit changed, executed against the code and the test it describes, above.
- The final palette sweep over the 22 files. True and checked include `color.rs:106` to `:110`, `:125` to `:126`, `:148` to `:152`, `:230` to `:236`, `:267` to `:270` and `:287` to `:291`, `frame_open.rs:409` to `:410` and `:540` to `:542`, `frame-open.md:133` to `:135` and `:147` to `:150`, `pixel-pipeline.md:456` to `:503`, `colour.rs:132` to `:135` and `:35` to `:36`, `cpu.rs:97` and `:1050` to `:1053`, test docs `palette.rs:305` to `:320`, `:364` to `:377`, `:393` to `:413`, `:434` to `:445`, `:492` to `:494` and `:501` to `:507`, `frame_open.rs:1055` to `:1062` and `:1245` to `:1249`, `tests/colour.rs:743` to `:754` and `:778` to `:780`, plan items 8 and 11, `:343`, `:409`, floor rows `:494` to `:496` and R1 at `:572`. Plan item 8 at `:546` says "decided from the header" and is refined by R1 in its own paragraph, the record of the answer as given. `tests/colour.rs:744`'s block shows `>> 8` beside entries above 255 and gives no false cause.

## Limitations and handoff

- No wasm build, rustdoc, mutation or corpus run this pass. The commit changes only comments and plan text, and pass 4's receipts for those stand.
- The viewer half was not reviewed.
- Next owner: the lead or implementer for D1, one comment. Then a fresh independent pass. This pass has a blocking finding, so the library half does not pass.
