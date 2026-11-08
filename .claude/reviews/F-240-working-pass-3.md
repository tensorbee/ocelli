# F-240 review, pass 3

**Reviewed**: `work/f-240-xai` range `4f9a7850..6f2ff0ca` at tip `6f2ff0ca`,
the single commit "F-240, review pass 2 remediation in the library". No
unstaged tracked path. The only untracked paths are the `.venv` and
`tools/oracle/out` setup symlinks.
**Author / reviewer**: remediation by GLM in opencode, with self-review notes
`.claude/scratch/F-240-self-review-5.md` to `-7.md`. Reviewed by Claude
(Opus) as an independent `/microscope --working` pass. The reviewer wrote
none of the reviewed material and did not write pass 1 or pass 2.
**Result**: 2 defects, 0 smells, 4 nitpicks (1 new, 3 carried)

## Pass 2 findings replayed

| Finding | Replay | State |
|---------|--------|-------|
| D1 build-targets tip size | `bin/ocelli.sh gate wasm` on a `git archive` copy of the tip: render 2,182,144, decode 16,455. 2,163 over base 2,179,981 is 0.0992 percent. The 96 bytes were attributed by removing the three `clamp(0.0, 1.0)` calls in `apply` from the tip copy and re-running the gate: 2,182,048, so the clamps are exactly the 96 bytes the LLD names | closed. Naming nit, N1 |
| D2 palette tolerance | Re-derived from the s15Fixed16 words of `R_XYZ`, `G_XYZ` and `B_XYZ` over 65,536 times the `f64` inverse of the note's printed matrix: `254.99970 187.51685 136.96207`. The assert `(254.9997, 187.5169, 136.9621)` within 1e-3 matches. The pipeline replies `254.99971 187.51683 136.96205`. M23 (`encoding_range` returns `2^n`) in scratch: the palette fixture goes red, `was (254.99799, 187.51556, 136.96109)`, green 1.34e-3 outside, and 12 of 16 integration tests go red in all | closed. Two comment figures, D2 below |
| D3 Annex F.3 | ICC.1:2022 fetched from `color.org/specification/ICC.1-2022-05.pdf` (title page "ICC.1:2022, Profile version 4.4.0.0"). Annex F (normative) "Profile computational models", F.3 "Three-component matrix-based profiles", equation F.6 rendered from page 102: connection XYZ equals the matrix whose columns are `redMatrixColumn`, `greenMatrixColumn` and `blueMatrixColumn` times linear r, g, b, and F.7 is its inverse. The note (`color.org/chardata/rgb/sRGB.pdf`) says "Combining this matrix with the inverse of the matrix from XN, YN, ZN to RL, GL, BL in A.7 results in the following matrix, which can be used for ICC Matrix/TRC sRGB profiles" after "The recommended chromatic adaptation matrix from D65 to D50". All three rewrites match both sources | closed |
| D4 residue figures | numpy: largest entry deviation 1.340e-5 (`f64`), 1.335e-5 (`f32`). Largest vertex move 2.662e-5 at `1 1 1`, green. Adobe red row green: 1.0725e-4 quantised against 1.0565e-4 decimal, 1.60e-6 apart, `12.92 * 255 = 3294.6`. Fixture dump: 0.002609 colourant-only, 0.002853 parametric, 0.005377 Adobe | closed |
| D5 window wording | The new sentence says 0 windows to 64 and -1311.9 to 0, 64 codes apart, both storing 0. Matches the LINEAR window at centre 64 width 256 and the `[64, 255, 64]` assert | closed |
| D6 semicolons | `git diff 4f9a7850..6f2ff0ca` added lines grepped for `;`, U+2014 and U+2013: only two code lines. Every comment line in `src/icc.rs`, `tests/icc.rs` and `src/color.rs` grepped for `;`: none. The whole story's added comment lines, `6163b82f..6f2ff0ca`: none | closed |
| N1 two products | `-0.97842103 * 0.60974 + 1.9158427 * 0.31111 + 0.033399127 * 0.01947 = 1.0566457e-4` and `3.8084170e-5`, encoded `0.348122` and `0.125472`. The constants are `PCS_TO_SRGB` row 2 and 3 as written | closed |
| N2 0.0027 | Now `0.0029`, split `0.0026` and `0.0029`, both reproduced by the dump | closed |
| N4 viewer-part sentence | Removed from `color.rs` | closed |

## Acceptance challenged

- **AC12, the transicc record, reproduced independently.** A scratch test
  built the six profiles with the test file's own `profile`, `xyz`, `curv`
  and `para` builders (gamma 1.0, gamma `0x0233`, the two-entry table,
  `para` type 3, the colourant swap, the Adobe colourants with `0x0233`),
  wrote them as `.icc`, and dumped Ocelli's `f32` replies through
  `icc_route` and `map_into` at 8 and 12 bits over 22 rows each. The same
  device values went to `transicc -t1 -n -i <profile> -o '*sRGB'`, once
  unbounded, once with `-s`, once with `-c0`. Results: in gamut 0.005519 at
  the two-entry table, 12-bit code 102 (device 6.3516), red, which is
  GLM's `0.0055` row. Under `-s` 0.353477 at the Adobe red row's green, the
  `0.3535`. `-c0` changes nothing. Little CMS prints the red row as
  `295.3368 -0.0077 0.0233` unbounded and `255.0000 0.0000 0.0233` with
  `-s`. Ocelli replies `254.99998 0.35348 0.12565`.
- **Whether the 0.3535 is real, and its cause.** It is real and it is
  Ocelli's. Adobe RGB (1998) and sRGB share the red chromaticity
  `0.64, 0.33`, so the colorimetric answer for Adobe red in sRGB is green
  and blue exactly 0, with red `1.398` linear. Inverting a Bradford D65 to
  D50 sRGB built from the primaries gives green `-3.9e-6` and blue
  `7.0e-6` from the Adobe decimals. Inverting the note's printed matrix,
  which Ocelli uses, gives `1.0564e-4` and `3.808e-5`. So the note's
  recommended adaptation and the Bradford adaptation the Adobe colourants
  carry do not cancel, and Ocelli lands 1.06e-4 linear above the true zero
  while Little CMS lands 2.3e-6 below it. The colourant quantisation adds
  only 1.6e-6 of the 1.07e-4. This is the design's ruled matrix doing what
  the design says, so it is not a wrong implementation, but it is a
  measured divergence in which Ocelli is the one further from the
  colorimetric answer, and the blue channel of the same row diverges by
  0.102.
- **Whether it reaches a stored byte or a displayed pixel.** No stored byte
  at the identity window: `0.3535` and `0` both store 0. But tier C's real
  caller, `ocelli-render/src/cpu.rs:937` `map_colour`, applies the D-29
  `ColourWindow` to the unrounded `map_into` output before `rgb_bytes`. A
  scratch probe through `ColourWindow::new(c, w, false).map_into` on
  Ocelli's red row against Little CMS's: width 20 at centre 10 gives
  `[255, 5, 2]` against `[255, 0, 0]`, width 8 at centre 4 gives
  `[255, 13, 5]` against `[255, 0, 1]`, width 2 at centre 1 gives
  `[255, 90, 32]` against `[255, 0, 6]`. So a narrow colour window shows it.
  The file's own D1 fixture says exactly this about a different value, that
  the window sees what the byte rule cannot. D1 below.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-pixel -p ocelli-render --test icc --test colour --lib` | `git archive 6f2ff0ca` in scratch, own target | now | exit 0. icc 16, pixel lib 24, render lib 124 (3 ignored), colour 9 (2 ignored, GPU) |
| `cargo fmt --all --check`, `cargo clippy -p ocelli-pixel --all-targets -- -D warnings` | same copy | now | both clean |
| `bin/ocelli.sh gate wasm` | same copy | now | ALL GREEN, render 2,182,144, decode 16,455 |
| same gate, three clamps removed | same copy, mutated, restored | now | render 2,182,048 |
| M23, `encoding_range` returns `2^n` | same copy, mutated, restored | now | 12 of 16 red, palette fixture among them |
| palette row dump, tolerance forced to 1e-9 | same copy, restored | now | `254.99971 187.51683 136.96205` |
| `assert_close` dump over the suite | same copy, restored | now | max gaps 0.002609, 0.002853, 0.005377 |
| numpy over the note's matrix, the s15 words, a Bradford sRGB | scratch scripts | now | quoted above and in D2 |
| `transicc` 2.18 (reports "5.1 [LittleCMS 2.18]"), 6 profiles by 22 rows by 2 depths | profiles from the test builders | now | 0.005519 in gamut, 0.353477 under `-s` |
| ICC.1:2022 PDF and the sRGB note PDF | color.org, fetched now | now | F.3 equations F.6 and F.7 read from the rendered page, note text extracted |
| `python3 scripts/prose_check.py` | worktree | now | clean over 913 files |

## Defects

### D1, the AC12 record hides that the 0.3535 reaches a displayed pixel, and does not state it as a D14 divergence

**Where**: `docs/lld/pixel-pipeline.md:590` to `:596`.
**What**: the record says Ocelli's `+1.073e-4` encodes to `0.3535` while
Little CMS's `-0.0077` clips to 0, "the same near-zero class pass 1
recorded. No stored byte differs on any row, so nothing needed
investigation." Three problems. First, the D-29 colour window on tier C acts
on the unrounded value before the byte, so a narrow window shows the
divergence: 5 codes at width 20, 13 at width 8, 90 at width 2, measured
above through `ColourWindow::map_into`. "No stored byte differs" is true
only at the identity window and the sentence offers it as the reason no
consequence exists. Second, the cause is not named. The two engines use
different PCS to sRGB matrices, Ocelli the note's printed one, and since
Adobe red and sRGB red share a chromaticity the right answer is 0, so
Ocelli is the side further from it. Third, `-0.0077` is in codes of 255 and
`+1.073e-4` is linear, set side by side in one clause, so a reader takes
Little CMS's linear value to be -0.0077, which would contradict "within
`1.1e-4` linear of zero".
**Why it is wrong**: decision D14, a divergence is claimed as measured,
with its cause, never left implied to be inconsequential. Microscope
section 3. AC12 sets "above one code is investigated" as the trigger for
investigation, so "nothing needed investigation" may stand as AC12's
threshold, but not as a statement that nothing visible follows.
**Evidence**: the transicc run and the window probe, both under Acceptance
challenged.
**Scope and regression**: prose. Record it as a measured D14 divergence:
the figure, the cause (the note's matrix against Little CMS's built-in
sRGB, with Adobe red on sRGB's red primary so the colorimetric answer is
0), the same row's blue at 0.102, both values in the same unit, and that no
byte differs at the identity window but a non-identity D-29 colour window
acts on the unrounded value and shows it, with one measured window. Replace
"so nothing needed investigation" with "below AC12's one-code
investigation threshold".

### D2, two figures in the palette fixture's comment do not reproduce

**Where**: `crates/ocelli-pixel/tests/icc.rs:906` to `:910`.
**What**: "maps the normalised row to linear `0.99999720, 0.50000494,
0.25000753`". The s15 words over 65,536 times the printed inverse give
`0.99999733 0.50000491 0.25000752` in `f64`, and `0.99999723` with the
`f32` constant. No reading gives `0.99999720` or `0.50000494`. Then
"the largest gap the `f32` pipeline itself leaves on this row, `7.7e-5` at
the green". The `f32` gap between the reply `187.51683` and the asserted
`187.5169` is `7.63e-5`, and of that only `2.0e-5` is the pipeline's own,
against the unrounded hand value `187.51685`. The rest is the assert's
four-decimal rounding.
**Why it is wrong**: microscope section 3, a stated figure that no
execution gives, the class pass 2 counted as D4. No asserted value is
affected. The encoded tuple and the 1e-3 bound are right, and "about
thirteen" still holds.
**Evidence**: `pal.py` in scratch, quoted under Pass 2 findings replayed.
**Scope and regression**: one comment. Print `0.99999733, 0.50000491,
0.25000752`, and either say `7.6e-5` against the rounded assert or `2.0e-5`
against the hand value, naming which.

## Smells

None.

## Nitpicks

- N1, new. `docs/lld/build-targets.md:347` says "the pass 2 remediation
  added" the 96 bytes, and `:340` "at the pass 1 state". The commits are
  titled "review pass 1 remediation" (`72a75882`, which added the 96 bytes)
  and "review pass 2 remediation" (`6f2ff0ca`, which added none). Naming
  the commit, or "the remediation after pass 1", removes the off-by-one
  reading. The figure and the attribution are right, measured above.
- N3, carried. `rendering_intent` still has no test.
- N5, carried. Table 68 parameters NOTE 1 leaves undefined still produce
  NaN, which the byte rule stores as 0.
- N6, carried and unconfirmed. The v2 profile padding rule was not read.

## Verified clean

- All nine replayed pass 2 items above, each through the tip copy.
- D3 against the primary sources, both fetched this pass: ICC.1:2022 Annex
  F.3, F.6 and F.7, and the sRGB note's derivation sentence, its A.7 clip
  note and its printed matrix.
- M23 red through the palette fixture with the ruled tuple and bound.
- The build-targets figures, and the 96-byte attribution by measurement,
  not by `twiggy`.
- Every changed figure in the module comment, the D1 fixture doc, the
  palette doc and the two LLD paragraphs, except the two in D2.
- AC12's `0.0055` and `0.3535`, the tool version string, and the command
  shape. The in-gamut maximum depends on the row set: over my 22 rows it is
  the same `0.005519` once code 102 at 12 bits is included and `0.00126`
  without it.
- Tier C ordering of ICC, window and byte in `cpu.rs:924` to `:940`.
- No comment semicolon, em-dash or en-dash anywhere in the story's added
  comment lines. `prose_check.py` clean. fmt and clippy clean.
- README, `use.html` and `more.html` are not in the range, and the range
  changes no behaviour, only comments and docs.

## Limitations and handoff

- No GPU, no oracle. Two GPU render tests stay ignored.
- IEC 61966-2-1 itself was not read. ICC.1:2001-04 (v2) was not read.
- Little CMS prints four decimals, so two of my rows at `207.5` (12-bit code
  2569, gamma 1.0 and swap) sit on a half-code tie that the printout cannot
  resolve. They are not counted as byte differences.
- Little CMS was run as a host tool only. Its source was not read.
- GLM's Python profile builder was not seen. Mine used the test file's own
  builders, and the two recorded figures reproduce from them.
- Scratch copy, profiles and both PDFs deleted with Python `shutil` after
  the pass.
- Still owed, not closed by this commit: AC9, AC14 to AC16, and the viewer
  part. Next owner: the implementer fixes D1 and D2, then a fresh pass 4.

## Self-review catch figure

All three new findings were within reach of what self-reviews 5 to 7 say
they did. D1: the self-review measured the `0.3535` and the D1 fixture's own
comment, edited in this commit, says the colour window sees what the byte
rule cannot. D2: cycle 1 says it recomputed the linear triple and the gap.
N1: the commit titles are one `git log` away. **The self-review should have
caught 3 of 3** new findings, both defects among them.
