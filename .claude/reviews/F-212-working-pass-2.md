# F-212 review, pass 2

**Reviewed**: range b236d6f2..84880ca7 on `work/f-212-claude`, one commit, 84880ca7 "F-212, review pair B fixes". HEAD was 84880ca7 and the worktree was clean apart from the untracked `.venv` and `tools/oracle/out` links
**Author / reviewer**: the implementer of the pair B fixes wrote the range. The reviewer is an independent Claude reviewer who wrote neither the implementation nor the fixes nor the pair B review
**Result**: 0 defects, 0 smells, 2 nitpicks

## Acceptance challenged

The range answers pair B's D1, S1 and S2 and corrects the plan's quadruped list. Each was replayed through the real code, not read from the diff.

**D1, README step 8.** The corrected sentence says the bar reads `at the detector` "when that spacing is at the detector: Imager Pixel Spacing with no magnification factor, alone or with a Pixel Spacing equal to it". A probe test in a scratch copy called `choose_spacing` in `examples/folder-viewer/src/measure.rs` for 17 input combinations. `detector` came back for exactly three of them. Those were Imager Pixel Spacing alone, a non-square Imager Pixel Spacing alone, and a Pixel Spacing equal to the Imager Pixel Spacing with no factor. Every other row gave a source the sentence does not cover with those words, as follows.

| Inputs | Source | Words on the bar |
|--------|--------|------------------|
| Pixel Spacing alone, with or without a factor | `pixel-spacing` | none |
| Pixel Spacing that differs from Imager Pixel Spacing, with or without a factor | `pixel-spacing` | none |
| Pixel Spacing equal to Imager Pixel Spacing, no factor | `detector` | `at the detector` |
| Pixel Spacing equal to Imager Pixel Spacing, factor 2 | `imager-spacing`, 0.1 | none |
| Pixel Spacing equal to Imager Pixel Spacing, factor unusable or 0 | refused | no bar |
| Pixel Spacing unusable beside Imager Pixel Spacing | refused | no bar |
| Pixel Spacing beside an unusable Imager Pixel Spacing | `pixel-spacing` | none |
| Imager Pixel Spacing alone, square or not | `detector` | `at the detector` |
| Imager Pixel Spacing with factor 2 or 1 | `imager-spacing` | none |
| Imager Pixel Spacing with an unusable factor | refused | no bar |
| A factor alone | refused | no bar |
| Nothing | `uncalibrated` | no bar, because no column spacing |

The page appends the words on `calibrationSource === "detector"` at `page.html` line 2628, so the sentence matches the code for every source. A refused file gives no bar, which the sentence before it already covers with "a spacing a length can use".

**S1, the leap year row.** By hand, under the plan's age rule (plan line 144) and PS3.5 6.2 AS, 2000-12-15 to 2001-01-05 gives years 2001 - 2000 = 1, less one because (1, 5) is before (12, 15), so 0. Months are 1 * 12 + 1 - 12 = 1, less one because day 5 is before day 15, so 0. Days are 31 - 15 = 16 to the end of December and 5 more, 21. The row expects `21 days` and the test comment says the same. The row discriminates the year loop because the birth's count takes 2000's February from the month loop while the study's count takes all of 2000 from the year loop.

**S2, the calibration pair.** PS3.3 10.7.1.3 gives Pixel Spacing as adjacent row spacing then adjacent column spacing, so `0.25\0.5` is row 0.25 and column 0.5, as the new comment says. The bar is horizontal and reads `calibrationColumn` at `page.html` line 2247 and in `paintScale`, so "The bar reads the column" is true.

**Plan line 109.** PS3.3 C.7.6.1.1.1 gives the quadruped caudal term as `CD`. The change from `CA` is correct.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer` | worktree at 84880ca7 | now | exit 0. lib 73, body_form_budget 1, measure 14, preview 87, scheduled_decodes 1, stalled_reader 1, value 24 |
| `cargo clippy -p ocelli-folder-viewer --all-targets -- -D warnings` | same | now | exit 0 |
| `cargo fmt -p ocelli-folder-viewer --check` | same | now | clean |
| `python3 scripts/prose_check.py` | same | now | voice rules clean over 908 files |
| scan of the range's added lines for en-dash, em-dash and semicolon in README and plan prose | `git diff b236d6f2..84880ca7` | now | none |
| `choose_spacing` probe, 17 combinations | scratch copy of 84880ca7, own target dir | now | table above |
| mutation catalogue, `--test preview` per mutant | scratch copy, each edit asserted present once and reverted | now | table below |
| pair B's original M14 and M20 survivals | `.claude/reviews/F-212-pair-B.md` | prior receipt | both survived at b236d6f2 |

| Mutation | Result |
|----------|--------|
| A1 `day_number` year loop adds 365 for every year | red, `computed_age_is_the_age_at_the_study`, 20001215 to 20010105 gave 20 days |
| A2 year loop asks `leap_year(earlier + 1)` | red, same row, 20 days |
| A4 year loop treats 2000 alone as 365 days | red, same row, 20 days |
| A1 with the new row deleted, control | green, so the new row is what catches it |
| C1 `X-Ocelli-Calibration-Column` writes `{row}` | red, `the_scale_bar_reads_the_length_tool_calibration`, column 0.25 |
| C2 `X-Ocelli-Calibration-Row` writes `{column}` | red, row 0.5 |
| C3 the pair swapped where `serve.rs` copies it into `Grey` | red, row 0.5 and column 0.25 |
| C4 the pair swapped at its source in `preview.rs` | red, the non-square table row, `Some((0.5, 0.25))` |
| C1 with the new header block deleted, control | green, so the new block is what catches it |

## Defects

None.

## Smells

None.

## Nitpicks

- N1, `README.md` line 267 is 103 characters where the surrounding paragraph wraps near 77. The fix appended the old line's tail to the new text rather than rewrapping. It renders the same.
- N2, the page's `tool-length` help sentence at `examples/folder-viewer/src/page.html` line 908 says "millimetres at the detector from Imager Pixel Spacing alone", the same wording pair B's D1 corrected in README step 8. It came from F-211 (333d1fe9), is outside this range and is not one of the three always-in-scope documents. It is a tooltip of one line where README step 9 gives the equal-pair rule in the next sentence. It is recorded for F-211's owner rather than counted here.

## Verified clean

- D1 closed. README step 8 agrees with `choose_spacing` for every calibration source, including the equal pair with a factor, the unusable factor and the unusable Imager Pixel Spacing beside a Pixel Spacing.
- S1 closed. The new age row is hand-derived, cites the rule its comment walks through, and goes red under three different leap year mutations of `day_number`. Without it the first mutation is green again.
- S2 closed. The new table row and the new header check together catch a swap at every one of the three places the pair is carried, the source in `preview.rs`, the copy in `serve.rs` and the header writer. Without the header block the header swap is green again.
- One step from S2, the non-square row also flows through `/measure`, and its JSON check expects `row_mm` 0.25 and `column_mm` 0.5.
- One step from D1, README step 9 and `use.html` line 697 already state the equal-pair rule and agree with the probe. `more.html` has no calibration sentence.
- The plan's `CD` correction agrees with PS3.3 C.7.6.1.1.1.
- The range adds no Rust outside the test file, no `as` cast, no `unsafe` and no dependency.
- The range's other added file is pair B's own review. Its D1, S1 and S2 claims were each reproduced above, including the 20 day mutant output and the `CA` against `CD` note.

## Limitations and handoff

- AC12 was not measured in a browser and the bar was not seen drawn on a non-square file. The page's use of the column value was read, not executed.
- No real DX or CR file was used. The probe and the mutations ran on synthetic objects.
- The scratch copy lived under the session scratchpad with its own `CARGO_TARGET_DIR` and is deleted with `shutil` after this pass. The worktree was not edited apart from this file, and nothing was committed.
- This pass is clean for the range b236d6f2..84880ca7. It does not stand for completed acceptance or for sprint readiness.
