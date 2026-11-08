# F-240 review, pass 4

**Reviewed**: `a765054b..90612371` at tip `90612371`, the single commit
"F-240, review pass 3 remediation in the library". It touches
`crates/ocelli-pixel/tests/icc.rs` (one doc comment), `docs/lld/build-targets.md`
and `docs/lld/pixel-pipeline.md`. No unstaged tracked path. The only untracked
paths are the `.venv` and `tools/oracle/out` setup symlinks.
**Author / reviewer**: remediation by GLM in opencode, self-review notes
`.claude/scratch/F-240-self-review-8.md` to `-10.md`. Reviewed by Claude
(Opus) as an independent `/microscope --working` pass. The reviewer wrote none
of the reviewed material and none of passes 1 to 3.
**Result**: 0 defects, 0 smells, 5 nitpicks (2 new, 3 carried)

## Pass 3 findings replayed

| Finding | Replay | State |
|---------|--------|-------|
| D1 AC12 record | Every figure re-measured, below. Cause, colorimetric answer, both units, blue `0.102`, identity-window sentence and the centre 10 width 20 window through `colour_bytes_into` all reproduce. "Nothing needed investigation" is gone | closed |
| D2 palette comment | `f64` linear `0.99999733 0.50000491 0.25000752`, `f32` linear `0.99999738 0.50000489 0.25000751`, pipeline reply `254.99971008 187.51683044 136.96205139`, `f64` encodings `254.999701 187.516855 136.962069`, `f32` literal gap `7.63e-5` at green, pipeline's own `2.41e-5`, `1e-3 / 7.63e-5 = 13.1` | closed |
| N1 commit naming | `d3fef64f`, `72a75882` and their quoted titles match `git log`. `git show 72a75882` adds the three `clamp(0.0, 1.0)` calls, `6f2ff0ca` adds none | closed |

## Acceptance challenged

- **AC12, the Adobe red row, re-measured.** The six profiles were rebuilt by a
  scratch test appended to a `git archive` copy of `tests/icc.rs`, using its
  own `profile`, `xyz`, `curv` and `para` builders, and written as `.icc`.
  `transicc -t1 -n -i adobe.icc -o '*sRGB'` on `255 0 0` prints
  `295.3368 -0.0077 0.0233`, with `-s` `255.0000 0.0000 0.0233`. Ocelli's
  `f32` reply is `254.999985 0.353477 0.125654` at 8 bits and at 12 bits
  (code 4095), so "at 8 and 12 bits alike" holds. Green gap `0.353477`, blue
  gap `0.125654 - 0.0233 = 0.10235`. Linear: Ocelli green
  `0.353477 / 255 / 12.92 = 1.0729e-4`, blue `3.814e-5`. Little CMS green
  `-0.0077 / 255 / 12.92 = -2.337e-6`, blue `7.07e-6`. Every value the record
  prints reproduces.
- **The cause, discriminated rather than read.** If the divergence were on the
  input side, Little CMS's route through the profile would differ from its
  route from the same XYZ. Feeding the quantised Adobe red colourant
  `0.609741 0.311111 0.019470`, times 100, to
  `transicc -i '*XYZ' -o '*sRGB'` prints `295.3368 -0.0077 0.0233`, identical
  to the profile route. Both engines therefore receive the same PCS value and
  differ only on the PCS to sRGB side. A numpy inverse of an sRGB built from
  the primaries with Bradford D65 to D50 gives green `-2.33e-6`, blue
  `7.08e-6`, encoded `-0.00767` and `0.02334`, matching Little CMS's print.
  The `f64` and `f32` products with `PCS_TO_SRGB` give `1.0727e-4` and
  `1.0729e-4`, matching Ocelli. The record's causal sentence is right.
- **The tier C window, through the real caller.** A scratch integration test
  in `ocelli-render/tests` parsed `adobe.icc`, attached it with `with_icc` to
  the interleaved native RGB route (8 bits, and 12 bits after
  `with_sample_scale`), took `RasterColor::supported` and called
  `colour_bytes_into`. Centre 10 width 20: `[255, 5, 2]` at both depths.
  Little CMS's `-s` triple through `ColourWindow::map_into` and the byte rule:
  `[255, 0, 0]`. The unbounded triple and the triple fed as stored samples
  through the plain RGB route give `[255, 0, 0]` too. Adjacent windows:
  centre 4 width 8 `[255, 13, 5]` against `[255, 0, 1]`, centre 1 width 2
  `[255, 90, 32]` against `[255, 0, 6]`, centre 127.5 width 255 (not identity)
  `[255, 0, 0]` both, centre 128 width 256 (`is_identity` true) `[255, 0, 0]`
  both. So the record's chosen window shows the divergence and the identity
  window does not, as stated.
- **The in-gamut and `-s` maxima over a wider row set.** 4,572 rows, every
  8-bit grey, every 12-bit code below 200 and every thirteenth above, the
  primaries, six profiles. The `-s` maximum is still `0.353477` at the Adobe
  red green. The in-gamut maximum rises to `0.0063` at the two-entry table's
  12-bit code 136, and 12-bit code 102 on that table gives `0.0055`, the
  record's row. The record states its 22-row set, so this is consistent.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-pixel -p ocelli-render` | fresh `git archive 90612371` in scratch, own `CARGO_TARGET_DIR` | now | exit 0, 351 passed, 0 failed, 80 ignored. icc 16, pixel lib 24, render lib 124 (3 ignored), colour 9 (2 ignored, GPU), colour_bytes 8 |
| `cargo clippy -p ocelli-pixel -p ocelli-render --all-targets -- -D warnings`, `cargo fmt --all --check` | same copy | now | both clean |
| `python3 scripts/prose_check.py` | worktree | now | clean over 914 files |
| `python3 scripts/deviation_check.py` | worktree | now | 31 deviations, every citation resolves |
| added-line grep for `;`, U+2014, U+2013 | `git diff -U0 a765054b..90612371` | now | none. The one `-` hit is `2^16 - 1`, arithmetic |
| story comment-line grep, same markers | `git diff -U0 6163b82f..90612371`, `//` lines in `.rs`, all `.md` lines | now | none |
| non-comment `.rs` lines in the range | `git diff a765054b..90612371 -- '*.rs'` | now | none. No arithmetic, constant or assertion changed |
| Ocelli dump, six profiles, 4,572 rows | scratch test on the tip copy, removed after | now | as above |
| `transicc` 2.18 ("5.1 [LittleCMS 2.18]"), unbounded, `-s`, `-c0`, `*XYZ` input | profiles from the test builders | now | as above. `-c0` changes nothing on the red row |
| `colour_bytes_into` window probe | scratch `ocelli-render` test, removed after | now | as above |
| palette reply print | scratch `println!` in the palette fixture copy, removed after | now | `254.99971008 187.51683044 136.96205139` |
| numpy over the s15 words, the printed matrix, `PCS_TO_SRGB` and an emulated `f32` `product` | scratch scripts | now | as above |

## Defects

None.

## Smells

None.

## Nitpicks

- N7, new. `docs/lld/pixel-pipeline.md:607`, "No stored byte differs on any
  row at the identity window" is true of the 22 rows and not a property of
  the two engines. Over the wider row set, the two-entry table's 12-bit code
  74 stores Ocelli's `36.502766` as 37 and Little CMS's `36.4979` as 36 on all
  three channels, a `0.005` gap straddling a half code. Below AC12's
  one-code threshold, so no AC is affected. "on any of these rows" would stop
  a reader generalising. The clause "which is below AC12's one-code
  investigation threshold there" also hangs off "no byte differs" where it
  means the `0.3535` gap.
- N8, new and outside this range. `crates/ocelli-render/src/cpu.rs:110` and
  `:356` still say `RasterColor::Unsupported` is ICC, while
  `RasterColor::supported` accepts an ICC transform and `colour_bytes_into`
  draws it, measured above. `cpu.rs` is untouched by F-240, and the wiring is
  the owed viewer part, so this is for the lead to route there rather than a
  finding against this commit. Cosmetic in the same file family:
  `pixel-pipeline.md:596` and `:602` break the paragraph's wrap.
- N3, carried. `rendering_intent` still has no test.
- N5, carried. Table 68 parameters NOTE 1 leaves undefined still produce NaN,
  which the byte rule stores as 0.
- N6, carried and unconfirmed. The v2 profile padding rule was not read.

## Verified clean

- Every figure in the new AC12 paragraphs: `0.3535`, `+1.073e-4`, `-2.3e-6`,
  `-0.0077`, `0.0000`, `1.1e-4`, `0.102`, `+3.8e-5`, `+7.1e-6`, the 8 and 12
  bit claim, `[255, 5, 2]` and `[255, 0, 0]`, the `0.0055` row, and the row
  count arithmetic `9 + 3 + 7 + 3 = 22`.
- The cause sentence, by the `*XYZ` route and the Bradford reconstruction.
  The colorimetric zero follows from the shared red chromaticity `0.64, 0.33`.
  The Bradford inverse on the Adobe published decimals gives `-3.9e-6`, the
  residue of their five-digit rounding.
- `map_colour` windows the unrounded `map_into` output before `rgb_bytes`,
  `cpu.rs:949` to `:961` and `:938` to `:940`.
- All six D2 figures and the "about thirteen" ratio. The `1e-3` bound and the
  asserted tuple are unchanged.
- N1's two commit references and titles.
- No stale copy of the removed figures (`0.99999720`, `7.7e-5`, "Twenty device
  rows", "nothing needed investigation") anywhere in the tracked tree.
- README, `use.html` and `more.html` are not in the range and the range
  changes no behaviour.

## Limitations and handoff

- No GPU, no oracle. Two GPU colour tests stay ignored.
- GLM's own row set and `run_transicc.py` were not seen. The record's 22-row
  figures were reproduced at the named rows, and the maxima were checked over
  a superset.
- Little CMS run as a host tool only, source never read. dwv, Horos and Grok
  not opened.
- Scratch copy, profiles, dumps and scripts deleted with Python `shutil`.
- Still owed, not closed by this commit: AC9, AC14 to AC16, the viewer part,
  and the matrix backlog story the lead ruled for sprint close. This pass is a
  clean bounded review of the range, not acceptance of the story.

## Self-review catch figure

No new defect or smell. Of the two new nitpicks, N7 needed a row set wider
than the self-review's, and N8 is outside the reviewed files. **The
self-review should have caught 0 of 0** blocking findings, and it did close
all three pass 3 items correctly.
