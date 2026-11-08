# F-239 review, pass 6, library half of the viewer part

**Reviewed**: `work/f-239-lib`, range `HEAD~1..HEAD` (one commit, `4b306efe`, "F-239, library half pass 5 comment fix"), worktree `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-239-lib`, clean apart from untracked `.venv` and the `tools/oracle/out` symlink.
**Author / reviewer**: range written by the lead remediating `.claude/reviews/F-239-lib-working-pass-5.md` D1. Reviewed by an independent Claude reviewer under `/microscope F-239 --working`, pass 6, a short pass scoped to that one comment and its test function's immediate neighbours. The reviewer wrote none of this range and none of passes 1 to 5.
**Result**: 0 defects, 0 smells, 0 nitpicks.

F-239-lib pass 6: 0 defects, 0 smells, 0 nitpicks.

## Acceptance challenged

- **Pass 5 D1, `crates/ocelli-pixel/tests/palette.rs:223` to `:225`.** The new comment says entry 255 is `255 * 257 = 65535`, every channel holds entries above 255, so ruling R1 does not read the palette as 8-bit, and the 16-bit descriptor shifts it right by 8 to 255. Checked against the test's own palette: `wide` is `index * 257` for `index` in `0..256` with `ENTRIES = 256` (`palette.rs:23`), so every entry from index 1 up is at least 257, and all three channels are built from `wide` declared 16 bits. `PaletteColorLut::new` (`crates/ocelli-pixel/src/color.rs:127` to `:138`) passes each descriptor's `bits_per_entry`, 16, to `with_entry_bits`. There `any_wide` is true and the all entries at most `255.0` test fails, so `read_as_8_bit` is false and `scales` is the declared scales (`color.rs:180` to `:189`). The declared 16-bit scale is the item 11 shift of `n - 8 = 8` (`color.rs:287` to `:288`). `65535 >> 8 = 255`, which the test asserts. The sentence now names the cause pass 5 asked for, and "the 16-bit descriptor shifts it" is true once the palette is not read as 8-bit, since the declared scale is the descriptor's. Closed.
- **Neighbours.** Within `bits_per_entry_comes_from_the_descriptor_and_not_from_the_data`: the doc at `:192` to `:195` is about the entry container deciding range, and the 8-bit declaration of the same data is refused as `LutValueOutOfRange`, which the test asserts, so it holds. "Declared 16-bit, so 65,535 is in range" holds. "Entry 1 is 257, and 257 >> 8 = 1" follows from the new comment. The preceding test's comment at `:172` to `:175` already names entries above 255 as the cause. The following test, `palette_channels_that_disagree_on_their_descriptor_are_refused`, makes no shift claim. No sentence in these contradicts the rule.
- **Commit hygiene.** The `.rs` change is comment lines only. No em-dash, en-dash or semicolon in the added lines under `crates`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git show HEAD` | the range | now | three comment lines replace two in `palette.rs`, plus the pass 5 review file |
| `cargo test -p ocelli-pixel --test palette bits_per_entry` | worktree at 4b306efe | now | exit 0. 1 passed |
| `git diff -U0 HEAD~1..HEAD -- crates`, added lines for U+2014, U+2013 and the semicolon | the range | now | none |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Limitations and handoff

- No full sweep, wasm build, rustdoc, mutation or corpus run this pass. The commit changes one comment, and pass 5's sweep and receipts stand for the rest.
- The viewer half was not reviewed.
- Pass 5's N1 to N3 were not counted then and are not re-raised.
- This pass has no blocking finding, so the library half passes review at `4b306efe`.
