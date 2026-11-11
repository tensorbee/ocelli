# F-239 review, pass 1, library half of the viewer part

**Reviewed**: `work/f-239-lib`, range `05d9aca2..a40accc9` (699a5148, 01bc1342, 82a0a79e, a40accc9), worktree `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-239-lib`, clean apart from untracked `.venv` and `tools/oracle/out`. No shipped source changed after 01bc1342 (`git diff --stat 01bc1342..a40accc9 -- 'crates/*/src'` is empty).
**Author / reviewer**: written by a Claude implementer. Reviewed by an independent Claude reviewer under `/microscope F-239 --working`, pass 1. The reviewer wrote none of this range.
**Result**: 3 defects, 2 smells, 2 nitpicks. Four points need a lead ruling.

F-239-lib pass 1: 3 defects, 2 smells, 2 nitpicks.

## Acceptance challenged

- **AC19 and D-29, colour window and invert.** Derived from `tools/oracle/node_modules/@cornerstonejs/core/dist/esm/RenderingEngine/helpers/cpuFallback/rendering/getVOILut.js:1` to `:7`, `generateColorLUT.js:13` to `:21`, `renderColorImage.js:30` to `:36` and `storedColorPixelDataToCanvasImageData.js`. cornerstone computes `((x - (c - 0.5)) / (w - 1) + 0.5) * 255`, clamps with `Math.min` and `Math.max`, subtracts from 255 when inverted, and stores into a `Uint8ClampedArray`. `ColourWindow` builds `VoiTransform::new(None, &[c], &[w], 0, VoiFunction::Linear, 0, 255)` and applies `apply_window`'s LINEAR arm (`lut.rs:798` to `:809`), then `PresentationTransform` INVERSE (`ymin + ymax - y`, `lut.rs` `apply`) through the new crate-private `inverse_over`. No window or invert formula is written in `color.rs`. The clamps agree: at `x = lower` the formula is exactly 0 and at `x = upper` exactly 255, so `x <= lower` and `x > upper` give cornerstone's values. Every hand value in `tests/color.rs` and `tests/colour_bytes.rs` was recomputed here and agrees, including the width 10 row that rejects `w` for `w - 1` (`63.75 140.25 191.25`) and LINEAR_EXACT (`51 127.5 178.5`). The skip at width 255 or 256 and centre 127 or 128 with invert off matches `renderColorImage.js:30` to `:36`, which also requires `image.getCanvas()`, and `createImage.js:278` to `:279` defines that for every colour image.
- **AC21 under the operator's tie decision.** cornerstone's own value for the tie case, run in node: `((130 - 128) / 510 + 0.5) * 255` is exactly `128.5` in double, and `Uint8ClampedArray` stores 128, inverted `126.5` stores 126, `0.5` stores 0. The tip asserts 128 and 126, and the `127.5` case rounds up to 128, so both tie directions are discriminated.
- **Item 11 and AC11, AC22, the display scale.** PS3.3 C.7.6.3.1.5 read at dicom.nema.org, edition 2026d: the third value "shall take the Value of 8 or 16", the data is "stored in a format equivalent to 8 bits allocated when the number of bits for each entry is 8, and 16 bits allocated when ... 16", 8-bit entries in 16 bits allocated are detected by value length, and "Palette color values must always be scaled across the full range of available intensities". That last sentence supports the operator's rule that a 16-bit descriptor is scaled as 16 whatever its entries hold. Accepting 9 to 15 bits is the operator's widening, not the standard's, and the fixture says so.
- **Unchanged consumers of the changed palette provider.** `PaletteColorLut::lookup` now scales, so `ColourLut::palette`'s GPU bake (`colour.rs:134` to `:144`) and `RasterColor::supported` (`cpu.rs:122` to `:131`) both read scaled entries. Probed: a 16-bit palette with red entry 1000 bakes to texel `3 0 255 255`, where base refused it. Tiers A, B and C therefore agree with each other, but tier A and B output changed. See D2, S1 and R4.
- **AC27 against the corpus.** Both corpus palette rows were inspected with pydicom and drawn by the tier C candidate producer at base and at tip, each built in its own target directory. See R1.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-pixel -p ocelli-render -p ocelli-dicom` | worktree at a40accc9 | now | 0 failed, 547 passed, 83 ignored, 49 test binaries |
| `cargo test -p ocelli-render --test byte_ties -- --ignored --nocapture` | worktree at a40accc9, real adapter (the LLD names Apple M4 Max under Metal) | now | 2 passed. `round_half_even_byte against tier C: 765 values, 0 disagree`. `unorm8 store at 255 ties: 128 differ from tier C, 255 rounded up, 0 rounded down`, the 128 being every tie whose lower code is even |
| `bin/ocelli.sh check` for `ocelli-pixel`, `ocelli-render`, `ocelli-dicom` | worktree | now | exit 0 each |
| `bin/ocelli.sh gate wasm` | worktree tip | now | ALL GREEN. Render module 2,179,981 bytes, ceiling 2,258,755. Decode module 16,455 |
| `bin/ocelli.sh wasm` | scratch copy of 05d9aca2, own `CARGO_TARGET_DIR` | now | render module 2,179,477 bytes, so the delta is +504, as `docs/lld/build-targets.md` records |
| `cargo tree -p ocelli-wasm -e normal --target wasm32-unknown-unknown --prefix none`, with and without `--no-default-features --features decode-worker` | worktree | now | default reaches `ocelli-pixel` and `ocelli-render` and no codec, dicom, openjph or ritk crate. Decode reaches `ocelli-pixel`, `ocelli-cache`, `ocelli-core` only. Matches the LLD |
| `ocelli-compare produce --reference <copy of tools/oracle/out> --corpus corpus/data`, tip, and tip with only the tie line reverted, separate trees and target dirs | scratch copies of a40accc9 | now | 102 views, 106,954,752 bytes, 0 bytes differ. The reverted tree turns `a_tie_rounds_to_the_even_channel_byte` red, so the builds differ |
| Same producer, base 05d9aca2 in its own target dir, against tip | scratch | now | only the two palette views differ, 634,212 and 649,735 bytes. Base draws both in colour, tip draws both black (0 non-zero RGB bytes) |
| Mutation catalogue, 18 mutations, `scratchpad/f239lib-p1`, own target dir | scratch copy of a40accc9 | now | 15 red, 3 survived. Table under "Verified clean" and S2 |
| Probe tests (`palette_color_lut` attribution, `ColourLut::palette` 16-bit bake) | scratch copy | now | D1 and D2 evidence below |
| Implementer's progress note, `.claude/scratch/F-239-lib-progress.md` | ignored file | prior receipt | "both palette candidates are all black at base and tip". **Not reproduced**, see R1 |

## Defects

### D1, the palette reader names the red descriptor for an entry out of range in another table

**Where**: `crates/ocelli-dicom/src/frame_open.rs:441` to `:447`, documented at `:420` to `:422` and in `docs/lld/frame-open.md:141` to `:143`.
**What**: `palette_color_lut` maps every `PixelError` from `PaletteColorLut::with_entry_bits` other than `UnsupportedColourDepth` to `attribute(PALETTE_DESCRIPTORS[0])`, which is `(0028,1101)`. The doc comment and the LLD say the error names "the descriptor or table that cannot be read, including ... an entry beyond its declared depth". A green table entry beyond a 12-bit depth is reported as the red descriptor.
**Why it is wrong**: a claim in prose that is false, and a refusal that points a user at an element that is valid. The 8-in-16 case for the same channel names `(0028,1202)` correctly, because it is refused earlier by `LutDescriptor::new` with the data tag, so the two refusals of one class disagree.
**Evidence**: probe in the scratch copy, descriptor `3\0\12` for all three, green table `0 4096 2`: `Err(Attribute { group: 40, element: 4353 })`, that is `(0028,1101)`. Descriptor `3\0\8`, green words `0 300 2`: `Err(Attribute { group: 40, element: 4610 })`, `(0028,1202)`.
**Scope and regression**: present on HEAD. Affects the folder viewer's refusal sentence and the oracle once AC27 moves it to this reader. Smallest fix: check `2^bits - 1` per channel inside `palette_channel`, where the data tag is known, or map `LutValueOutOfRange` per channel. Regression: the probe above as a fixture asserting `(0028,1202)`.

### D2, `ColourLut::palette` still says a 16-bit entry above 255 is not scaled

**Where**: `crates/ocelli-render/src/colour.rs:129` to `:133`.
**What**: the doc says the bake refuses "when a channel is not an integer in `0..=255`. A 16-bit entry above 255 is not scaled into the byte. That scale is not this story's formula." Since 01bc1342 the entry is scaled by `DisplayScale` inside `PaletteColorLut::lookup`, so no such refusal happens.
**Why it is wrong**: a false claim in prose on the GPU tier's palette entry point, HLD 27.3's human review item.
**Evidence**: probe, red `0 1000`, green `0 255`, blue `0 65535`, all 16-bit: `ColourLut::palette` returns `Ok([0, 0, 0, 255, 3, 0, 255, 255])`.
**Scope and regression**: present on HEAD. Delete the two sentences and say the entries arrive scaled by their descriptors. The fixture is S1.

### D3, `raster_color_into` still says a colour sample has no scale

**Where**: `crates/ocelli-render/src/cpu.rs:388` to `:389`.
**What**: "A stored colour sample outside 0 to 255 has no scale yet". The next line now calls `stored_colour_is_a_display_byte`, which tests the value after the transform's sample scale (`cpu.rs:1054` to `:1061`).
**Why it is wrong**: a false comment beside the arithmetic it describes. It invites a reader to remove the scale from the check, which M15 shows would refuse every 12-bit frame.
**Scope and regression**: present on HEAD, comment only. Reword to the scaled test.

## Smells

### S1, the GPU palette bake's changed output has no fixture

**Where**: `crates/ocelli-render/src/colour.rs:134` to `:144` and `bake` at `:350`, against `crates/ocelli-render/tests/colour.rs:125` to `:134`, which builds 8-bit palettes only.
**What**: tier A and B palette output changed for every 16-bit table (before, refused above 255 and drawn raw at or below it, and now shifted by 8). `tests/cpu.rs` asserts the tier C byte, `[3, 0, 0, 255]`, and its comment says "the GPU bake and this raster read the same display byte", but nothing asserts the bake's bytes. The plan's Boundary section (`.claude/plans/F-239-design.md:455`) says "The GPU colour path, `ColourLut` in `crates/ocelli-render/src/colour.rs`, is unchanged".
**Failure mechanism**: a later change that bypasses `PaletteColorLut::lookup` in the bake, for speed or to read raw entries, would make tiers A and B draw a 16-bit palette differently from tier C with a green suite, which is §31's quietly different result. CLAUDE.md requires a hand-computed fixture for every pixel arithmetic route.
**Smallest regression**: a `tests/colour.rs` fixture baking descriptors `n\0\16` with `0 255 256 32768 65535` and asserting texels `0 0 1 128 255`, the AC22 values.

### S2, three reader branches survive their mutation

**Where**: `crates/ocelli-dicom/src/frame_open.rs`, `palette_channel` and `palette_entries`.
**What**, each claimed in the doc comment and `docs/lld/frame-open.md`:
- M18, `stored_from_u16(first_raw, signed)` to `false`: green. The signed first mapped value under Pixel Representation 1 is not exercised.
- M20, entry count 0 read as 65,535 rather than 65,536: green. The reader's half of "0 means 65,536" has no fixture (a probe shows it works today, `entry_count()` 65,536).
- M11, the 8-in-16 container forced to 16: green. "an entry above 255 there is refused rather than read wide" still holds through `channel_scale`'s range check, but with D1's wrong element, so the documented route is not the one that refuses.
**Failure mechanism**: each is a branch that looks covered and is not, which is the defect class microscope item 4 names. `synthetic/sc_palette_color_16.dcm` uses count 0, but no test in this half reads it through `palette_color_lut`.
**Smallest regression**: one fixture per branch in `crates/ocelli-dicom/tests/frame_open.rs`.

## Nitpicks

- **N1.** `ColourWindow::apply`, `apply_in_place` and the window closure ship in the render module because `prepare_color` passes `ColourWindow::initial()` through `map_colour`, `cpu.rs:594` to `:603`. It is most of the 504 bytes, within budget, and `docs/lld/build-targets.md` attributes it honestly. A `raster_color_into` that skipped `map_colour`'s window step would let fat LTO drop it, as AC29 expected.
- **N2.** `frame_open.rs:445`, `UnsupportedColourDepth => PaletteDepth` is unreachable, because `palette_channel` refuses the same depths first at `:476`.

## Points needing a lead ruling

### R1, AC27 contradicts AC22 on the corpus, and the "black at base" premise is false

Both corpus palette rows declare 16 bits per entry and hold entries `0 ..= 255` only (pydicom: `synthetic/sc_palette_color.dcm` descriptor `256\10\16`, `synthetic/sc_palette_color_16.dcm` descriptor `0\0\16`, max entry 255, no entry above 255 in any table). Under item 11 every entry shifts to 0, so the tip draws both rows black. **Base draws both in colour**: built in its own target directory, base writes 634,212 and 649,735 non-zero RGB bytes for the two views, and the tip writes 0. cornerstone's reference also draws them in colour (637,198 non-zero RGB bytes in `synthetic__sc_palette_color.raw`), because no red entry exceeds 255. The implementer's note that both are black at base is not reproduced. This reviewer's own first base run shared `CARGO_TARGET_DIR` with the tip and silently ran the tip binary, giving the same false result, so that is the likely cause.
So AC27's "byte-identical before and after" for `sc_palette_color.dcm` cannot hold under AC22. PS3.3 2026d's "scaled across the full range" sentence supports the rule, which makes the corpus rows the outliers. The options are the lead's: regenerate both rows with full-range entries (entry times 257, or an 8-bit descriptor for the first), which is the codec part's file set (`scripts/corpus_synth.py`, `corpus/manifest.tsv`), and amend AC27, or keep the rows and record the change from colour to black as a measured divergence beside the floor table's palette row. Not a defect in this half, whose code follows AC22.

### R2, the operator's "monochrome moves too" has no monochrome byte to move

Tier C monochrome reaches `byte_from_channel` only with whole table bytes (`rgba_of_preset` and `preset_rgb` read a `ColourLut` texel). The monochrome tie decision is `display_index`'s `+ 0.5` address (`colour.rs:484`, ties up) and `reference_ramp_index`'s `round_half_even` (already ties to even). Re-measured: 0 of 102 candidate views change between the tip and the tip with ties up, so the implementer's measurement and the LLD's 0-byte claim stand. Whether `display_index` and the WGSL `table_index` should also round a tie to even is a lead ruling, because it changes tiers A and B together and the existing `display_index` comment says the reference quantises those tables with `Math.floor(value * 255 + 0.5)`. That comment also sits against the new `cpu.rs` test doc's "cornerstone3D 5.8.2's for every colour and monochrome byte it stores", which this pass did not confirm or refute in the vtk sources.

### R3, the render target's unorm8 store rounds every tie up

Reproduced on this adapter: 128 of 255 ties differ from tier C, all upward. It is the `present_display` mode 0 path, which emits a unit float with no table. No colour route on any tier reaches it, and tier C has no counterpart, as `docs/lld/render-graph.md` says. The driver owns the conversion, so it is measured and not asserted, which is correct under D14. Whether mode 0 should round in WGSL before the store is a lead ruling and not this half's defect.

### R4, lead-owned text the tie decision and the scale made stale

- `docs/hld/DEVIATIONS.md:46`, D-29: "Ties round up through tier C's shared byte rule where cornerstone rounds to even, measured by F-239's AC21". The tip rounds to even.
- `.claude/plans/F-239-design.md`: item 10, AC21 (expects 129), and the floor table's tie row, which the decision says moves to matching, still say ties up. The Boundary section's "`ColourLut` ... is unchanged" is false since 01bc1342 (S1, D2).
The plan and the register are written by the approving change and by neither part, so these are the lead's to amend.

### Implementer's five open points, placed

| Point | Finding | Owner |
|-------|---------|-------|
| AC27 against AC22, palettes black at base | AC27 contradicted, "black at base" false | R1, lead, then codec part (corpus) or viewer part (AC27 record) |
| Monochrome bytes unchanged by the tie rule | Re-measured, 0 of 102 views. True | R2, lead for `display_index` |
| `ColourWindow` in wasm, +504 bytes | Reproduced, gate green, attribution plausible | This half, N1 only |
| 16-bit palette reaches the GPU bake | Reproduced, consistent across tiers, untested and misdocumented | This half D2 and S1, plan line R4 |
| WGSL unorm8 store ties up | Reproduced, 128 of 255 | R3, lead |

## Verified clean

- **No second copy of the arithmetic.** `ColourWindow` calls `VoiTransform::apply` and `PresentationTransform::apply`. `inverse_over` builds the same struct `new` builds and adds no formula. `PresentationTransform::new` still refuses every colour space, so D-23's partition holds.
- **YBR conversion untouched.** No line of the matrices, offsets or 4:2:2 replication changed. `color.rs:1` still cites PS3.3 C.7.6.3.1.2. The sample scale is applied inside `pixel`'s `at` closure, before the matrix, and AC3's triple through Bits Stored 12 is asserted.
- **DisplayScale.** `from_bits` refuses outside `8 ..= 16`. `apply` is `floor(value / 2^shift)` through glam's `floor`, exact for every integer to `2^24`. Hand values `4095 >> 4 = 255`, `255 >> 8 = 0`, `32768 >> 8 = 128`, `65535 >> 8 = 255` recomputed.
- **Palette reading.** Packed 8-bit, 8 in 16 detected by length, odd count with one pad byte, OW words written back little-endian, segmented refused, missing refused, depth refused, `Ok(None)` for other photometric values. ICC presence at the top level only, nested in `(0048,0105)` false.
- **Half-to-even ties in tier C.** `byte_from_channel` finds the greatest code with `code - 0.5 <= value` and gives an odd code at an exact tie to the even one below. `lo - 1` runs only for an odd `lo`, so it cannot underflow, and `value` is positive there. The unit test holds ties of both parities with the float either side.
- **Casts.** No `as`, no `#[allow]`, no `unsafe` added. `u16::MAX >> (16 - bits)` and `1_u16 << shift` are bounded by the depth check.
- **Mutations that went red** (each run in the scratch copy, restored after): M1 shift `n - 7`, M2 sample scale dropped, M3 skip ignores invert, M4 width 255 dropped from the skip, M5 invert before window, M6 ties up, M8 palette depth from the container, M9 green by red's scale, M10 entry range unchecked, M12 odd pad, M13 segmented ignored, M15 display byte check unscaled, M16 window not applied in `colour_bytes_into`, M19 floor to round, M21 ICC never present.
- **LLD sentences executed.** `docs/lld/build-targets.md`: base and tip sizes, +504, both `cargo tree` reachability sentences. `docs/lld/render-graph.md`: the 0-byte tie measurement over 102 views, 765 WGSL values with 0 disagreements, 128 unorm8 differences at ties with lower even code. `docs/lld/pixel-pipeline.md`: a 16-bit table at or below 255 draws black, both corpus rows declare 16 bits, no scaled entry can fail the display byte test (entries are integers within range, `LutDescriptor::new` `lut.rs:85` to `:101`). `docs/lld/frame-open.md` is clean except the attribution sentence of D1.
- `README.md`, `use.html` and `more.html` are untouched in this range and their palette and stage 4 sentences are the viewer half's to update under the plan.

## Limitations and handoff

- AC17's full standards check was not done. Only PS3.3 C.7.6.3.1.5, edition 2026d, was read at dicom.nema.org. C.7.6.3.1.2, C.7.6.3.1.3, C.7.9, PS3.5 8.2.4 and A.4.4 and ISO/IEC 15444-1 Annex G were not.
- The cornerstone monochrome rounding claim in the `cpu.rs` test doc was not checked against the vtk sources (R2).
- The candidate producer used a copy of the main checkout's `tools/oracle/out` as the reference set. No oracle verdict was computed, and AC20 and AC27 records are the viewer half's.
- The folder viewer, the oracle's reader replacement and the user documents are the viewer half and were not reviewed.
- GPU evidence is from one adapter. A no-adapter machine would print and return.
- Next owner: the implementer for D1 to D3, S1 and S2, then a fresh independent pass. The lead for R1 to R4. This pass found blocking findings, so the library half does not pass.
- Scratch at `scratchpad/f239lib-p1` is removed with Python `shutil` after this file is written.
