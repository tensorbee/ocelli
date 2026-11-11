# F-239 review, pass 2, library half of the viewer part

**Reviewed**: `work/f-239-lib`, range `619fd87f..72066de0` (6ce63b86, 72066de0), worktree `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-239-lib`, HEAD `72066de0adb62e1d6e83b3a3abe22b1a3aa9b462`, clean apart from untracked `.venv` and the `tools/oracle/out` symlink. No unstaged paths were reviewed.
**Author / reviewer**: written by a Claude implementer remediating `.claude/reviews/F-239-lib-working-pass-1.md` under the plan's "Rulings after the library half's review pass 1" and the lead's later whole-palette ruling. Reviewed by an independent Claude reviewer under `/microscope F-239 --working`, pass 2. The reviewer wrote none of this range and did not write pass 1.
**Result**: 2 defects, 2 smells, 2 nitpicks.

F-239-lib pass 2: 2 defects, 2 smells, 2 nitpicks.

## Acceptance challenged

- **The whole-palette fallback, derived here.** PS3.3 C.7.6.3.1.5, current edition read at dicom.nema.org on 2026-11-07: the third value "shall take the Value of 8 or 16", "The third Value shall be identical for each of the Red, Green, and Blue Palette Color Lookup Table Descriptors", the note that 8-bit entries in 16 bits allocated are detected by value length, and "Palette color values must always be scaled across the full range of available intensities". cornerstone3D 5.8.2, `tools/oracle/node_modules/@cornerstonejs/dicom-image-loader/dist/esm/imageLoader/colorSpaceConverters/convertPALETTECOLOR.js:24` in the main checkout (package.json says 5.8.2): `shift = redDescriptor[2] > 8 && rData.some((num) => num > 255) ? 8 : 0`, applied to all three channels, stored through `Uint8ClampedArray` (`:2` to `:9`), so an unshifted entry above 255 saturates.
  - Conforming 16-bit, full range, every channel `0 25700 65535`: the standard and cornerstone both give `0 100 255`. Ocelli has an entry above 255, so no fallback, shift 8, `0 100 255`. Agree.
  - All three 16-bit with every entry at most 255, `0 100 255`: the standard gives `0 0 0`. cornerstone gives `0 100 255`. Ocelli falls back and gives `0 100 255` with the flag set, ruling R1. Asserted by `a_wide_table_whose_entries_fit_in_8_bits_is_read_as_8_bit_and_flagged` and, through the reader, `a_sixteen_bit_table_of_8_bit_entries_is_read_as_8_bit_and_flagged`.
  - Mixed, all 16-bit, red and green `0 100 255`, blue `0 100 256`: the standard gives `0 0 0 / 0 0 0 / 0 0 1`. Ocelli gives the same, because one wide entry above 255 makes the palette conforming. A per-channel decision gives `0 100 255 / 0 100 255 / 0 0 1`, red and green about 256 times too bright. cornerstone's red-only test gives `0 100 255` for all three, blue saturating at 255. The fixture asserts the standard's bytes, so it rejects both the per-channel rule and cornerstone's (M1 red).
  - Near-zero red in a conforming palette, red `0 100 255`, green and blue `0 25700 65535`: Ocelli `0 0 0 / 0 100 255`, the standard's. cornerstone `0 100 255 / 0 255 255`. This divergence is the plan's floor row "AC22 measures the case where the two differ", and it is the case the whole-palette ruling exists for.
- **Pass 1 replayed through the real code.** D1, the reader names its own table: `an_entry_beyond_its_depth_names_its_own_table` asserts `(0028,1202)` and `(0028,1203)`, and M9 (removing the reader's range check) turns it red, so the reader's check is the route and `channel_scale`'s is a backstop. D2, `ColourLut::palette`'s doc now says the entries arrive scaled. D3, `raster_color_into`'s comment now names the scaled test. S1, the bake fixture's texels recomputed here: red `0 255 256 32768 65535 >> 8 = 0 0 1 128 255`, green and blue `255 128 1 0 0`, texel `i = (r, g, b, 255)`, matching `colour.rs:756`'s 20 bytes, and the palette does not fall back because red holds 32768. S2: M11 (unsigned first mapped value) and M12 (count 0 read as 65,535) are now red. Pass 1's M11, the 8-in-16 container, has become equivalent (M13 here) because the container is now always 16 for words and the reader's range check refuses first.
- **R1 to R4.** R1: the code implements the fallback once for the whole palette. R2 and R3: `docs/lld/render-graph.md` records both rulings, and `cpu.rs`'s test doc no longer claims every monochrome byte. R4: D-29, item 10, AC21, the floor tie row and the Boundary sentence are corrected, but the fallback left other plan text stale, D1.
- **AC27 and R1 against the corpus.** Both rows draw as at the story's base, measured below.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-pixel -p ocelli-render -p ocelli-dicom` | worktree at 72066de0 | now | exit 0. 52 test binaries, 554 passed, 0 failed, 83 ignored |
| `bin/ocelli.sh gate wasm` | worktree at 72066de0 | now | exit 0, ALL GREEN. Render module 2,179,981 bytes, ceiling 2,258,755. Decode module 16,455, ceiling 17,277 |
| `ocelli-compare produce --reference <copy> --candidate <dir> --corpus corpus/data`, release build | `git archive` of 699a5148 (story base) and of 72066de0, each in its own tree under `scratchpad/f239lib-p2` with its own `CARGO_TARGET_DIR`. Binaries differ (sha1 `06455349` and `e8f3c33f`). Reference is a copy of the main checkout's `tools/oracle/out` | now | both `written 102, omitted 0, unsupported 2`. 102 views, 106,954,752 bytes, 0 differ. `sc_palette_color` 634,212 and `sc_palette_color_16` 649,735 non-zero RGB bytes on both sides, as `docs/lld/pixel-pipeline.md` states |
| Same tip candidate against the cornerstone reference, palette views only | as above | now | up to 64 codes on 2,986 and 2,943 channels. Identical at base, so not introduced by this range. Observation only, AC20 and AC27 records are the viewer half's |
| Mutation catalogue, 13 mutations, `scratchpad/f239lib-p2/tip`, own target dir, each restored and the tree diffed clean against the worktree after | scratch copy of 72066de0 | now | 9 red, 4 green. Table under "Verified clean", S1 and S2 |
| PS3.3 C.7.6.3.1.5 | dicom.nema.org, current edition | now | quoted above |
| `convertPALETTECOLOR.js` | main checkout `tools/oracle/node_modules`, 5.8.2 | now | the rule quoted above |
| `docs/lld/build-targets.md`'s "not byte-identical to 01bc1342's" | none | not executed | size equality re-measured, byte inequality not |

## Defects

### D1, the plan still describes a per-table, header-only palette rule and a changed 16-bit corpus row

**Where**: `.claude/plans/F-239-design.md:570` (R1), `:232` (item 11), `:343` (consumer table), `:404` (AC22), `:409` (AC27), `:494` (floor table palette row).
**What**:
- R1 says "When every entry of a table declared above 8 bits fits in 8 bits, the table is read as 8-bit". The code decides once for the whole palette (`color.rs:254` to `:271`), which is the lead's later ruling, and a per-table reading is exactly M1, which reads red and green of the mixed fixture 256 times too bright.
- Item 11 says "The rule is decided from the header, never from the data". The fallback is decided from the data.
- AC27 says `synthetic/sc_palette_color_16.dcm` "changes from its current outcome to AC22's scaled bytes", and `:343` says "recorded change for the 16-bit row". Under R1 both rows draw as at base, measured above, 0 bytes differ. R1 itself says so, so the plan contradicts itself.
- The floor row says Ocelli draws "every channel shifted by its own descriptor" and calls it better. For a palette whose wide entries all fit in 8 bits, which is both corpus rows, Ocelli now reads it as cornerstone does, and that case is the same, not better.
- AC22 names only `palette_entries_are_shifted_by_their_descriptor`, while its fallback rows are asserted by `a_wide_table_whose_entries_fit_in_8_bits_is_read_as_8_bit_and_flagged`.
**Why it is wrong**: false claims in prose, microscope item 3. Ruling R4 says stale text is corrected in this change, and the plan is what the viewer half and the sprint corpus gate read AC27's expectation from.
**Evidence**: the producer comparison above, and M1.
**Scope and regression**: present on HEAD, plan only. The plan is lead-owned under pass 1's R4, so the owner is the lead or the implementer at the lead's direction. Smallest fix: R1 and AC22 say "the whole palette", item 11 names the fallback as the one data-dependent exception, AC27 and `:343` expect both rows unchanged, the floor row gains the fallback case as Same.

### D2, the library's doc comments and the LLD still say a palette's depth never comes from the data

**Where**: `crates/ocelli-pixel/src/color.rs:109`, `:124` to `:127`, `:146` to `:148`, `:283` to `:289`, `crates/ocelli-dicom/src/frame_open.rs:409` to `:411`, `docs/lld/pixel-pipeline.md:464` to `:474`.
**What**:
- `with_entry_bits`: "The depth is decided from the header and never from the data." It now takes the identity when the data fit in 8 bits.
- `new`: "a 16-bit one is shifted right by 8". Not when the fallback is taken.
- `DisplayScale`: "For a palette entry it is the third value of that channel's ... Descriptor ... **The rule is decided from the header and never from the data**". Not for a palette that falls back.
- The `read_as_8_bit` field: "Ruling R1's fallback was taken for at least one channel." It is taken for all three or none.
- `palette_color_lut`: "Each channel is scaled to the display's 8 bits by its own declared depth".
- `docs/lld/pixel-pipeline.md:464`, in bold, "The depth comes from the header and never from the data", and `:473` to `:474`, "`PaletteColorLut::new` takes each channel's depth from its `LutDescriptor`, so ... a 16-bit one is shifted by 8". The paragraph at `:483` then says the opposite, so the LLD contradicts itself across one heading.
**Why it is wrong**: false comments beside pixel arithmetic, HLD 27.3's review item, which tells a human to check the arithmetic against the specification "not against the comment above it". A reader trusting `DisplayScale`'s bold sentence would remove the fallback as a bug.
**Evidence**: `grep -n 'never from the data\|16-bit one is shifted\|at least one channel' crates/ocelli-pixel/src/color.rs docs/lld/pixel-pipeline.md`, and M2 (removing the fallback turns two fixtures red, so the fallback is behaviour, not dead code).
**Scope and regression**: present on HEAD, prose only. Smallest fix: keep "from the header" for samples, name R1's whole-palette fallback as the one exception for palettes at each site, and fix the field doc.

## Smells

### S1, the flag's false state for an 8-bit palette has no fixture

**Where**: `crates/ocelli-pixel/src/color.rs:261` to `:262`, `any_wide`.
**What**: M4, `any_wide || true`, stays green. With it, every all-8-bit palette reports `entries_read_as_8_bit() == true`. Pixels are unchanged, because the scales are the identity either way, so only the flag is wrong.
**Failure mechanism**: ruling R1 says "the frame carries a flag the page states", and the viewer half will read this flag. A regression that sets it for 8-bit palettes would put the departure notice on every conforming 8-bit palette with a green suite. Every present assertion of `!entries_read_as_8_bit()` uses a palette with a 16-bit channel.
**Smallest regression**: assert `!entries_read_as_8_bit()` on an all-8-bit palette, AC7's palette in `tests/palette.rs` or the reader's 8-bit case in `crates/ocelli-dicom/tests/frame_open.rs`.

### S2, the disagreeing-descriptor culprit has no fixture

**Where**: `crates/ocelli-dicom/src/frame_open.rs:442` to `:456`, documented in `docs/lld/frame-open.md:144` to `:146`, "naming the first descriptor that differs from red's".
**What**: M7 (always green's descriptor) and M8 (always blue's) both stay green. `palette_file` writes one descriptor for all three channels, so no test reaches a mismatch through the reader. The closure compares `first_mapped_input` while `with_entry_bits` compares `first_input_bits`, which agree today because the input ramp starts at the first mapped value.
**Failure mechanism**: the branch was added by this remediation to fix pass 1's D1 class, a refusal pointing at the wrong element, and a later edit can reintroduce exactly that class unobserved. This is microscope item 4.
**Smallest regression**: one fixture per culprit, green with entry count 2 against red's 3 asserting `(0028,1102)`, and blue with a different first mapped value asserting `(0028,1103)`.

## Nitpicks

- **N1.** `docs/lld/build-targets.md:302` to `:303`: "the decode module is byte-identical to base. The decode module is byte-identical." The second sentence repeats the first.
- **N2.** `docs/lld/frame-open.md:149` is not reflowed, 130 columns where the file wraps at 78.

## Verified clean

- **Fallback arithmetic.** `read_as_8_bit` is computed once from every channel declared above 8 bits, after each `channel_scale` has bounded entries by `2^bits - 1`, and replaces all three scales together. `<= 255.0` is inclusive, M3 red. The mutation to per-channel, M1, and the fallback removed, M2, are red. M6, the flag forced false, is red. M5, letting 8-bit channels join the decision, is green and equivalent, because `channel_scale` already bounds an 8-bit channel to 255, so the `filter` is a statement of intent and not a branch.
- **Reader.** M9 (range check removed) and M10 (`>=` for `>`) red. M11 and M12 red. M13, the old 8-bit container for words, is green and equivalent, because the reader's range check refuses a wide word first and an 8-bit word is the same value in either container. `largest` is `u16::MAX >> (16 - bits)` with `bits` in `8 ..= 16`, so the shift is at most 8 and the value exact in `f32`. No `as`, `#[allow]` or `unsafe` added in the range.
- **Unchanged consumers.** `ColourLut::palette` reads `PaletteColorLut::lookup` through `resolve_palette` and `bake`, so the fallback reaches tiers A and B as it reaches tier C. `the_gpu_bake_reads_a_narrow_sixteen_bit_table_as_eight_bit`'s `10 200` texels match the identity.
- **D-29** in `docs/hld/DEVIATIONS.md` says ties round to even, asserted by AC21, and `a_colour_window_tie_rounds_half_to_even` exists at `crates/ocelli-render/tests/colour_bytes.rs:147`. D-29 says nothing about palettes, and nothing in it is stale.
- **LLD sentences executed.** `pixel-pipeline.md`'s producer measurement, 102 views, 0 bytes differ, 634,212 and 649,735. `build-targets.md`'s render size 2,179,981 and decode byte identity in size. `render-graph.md`'s R2 and R3 paragraphs against the rulings. `frame-open.md`'s own-table attribution against D1's fixture.
- `README.md`, `use.html` and `more.html` are untouched in this range, and the flag has no consumer yet outside tests.

## Limitations and handoff

- The byte inequality of the render module against 01bc1342 was not rebuilt. Only its size was re-measured.
- The palette difference against the cornerstone reference, up to 64 codes on about 2,950 channels per view, is present at base and was not attributed. It is the viewer half's AC20 and AC27 record.
- No GPU test was run in this pass. The bake fixtures read CPU-baked bytes and open no adapter.
- The viewer half, the oracle's reader replacement and the page's use of the flag were not reviewed.
- Next owner: the implementer for D2, S1 and S2, the lead or the implementer at the lead's direction for D1, then a fresh independent pass. This pass found blocking findings, so the library half does not pass.
- Scratch `scratchpad/f239lib-p2` is removed with Python `shutil` after this file is written.
