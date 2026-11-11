# F-240 review, pass 1

**Reviewed**: `work/f-240-xai`, range `6163b82f..d3fef64f`, the single commit
`d3fef64f` "F-240, matrix and tone-curve ICC profiles in ocelli-pixel, library
part". F-239's library branch underneath it was not judged. No unstaged
tracked path. The only untracked paths are the `.venv` and `tools/oracle/out`
symlinks created with the worktree at 14:49, before the commit at 18:20.
**Author / reviewer**: implemented by GLM in opencode with five self-review
cycles recorded in `.claude/scratch/F-240-self-review-1.md`. Reviewed by
Claude (Opus) as an independent `/microscope --working` pass. The reviewer
wrote none of the reviewed material.
**Result**: 8 defects, 1 smell, 5 nitpicks

## Acceptance challenged

- **AC12, transcription.** Every byte offset, signature, type layout and
  formula in `icc.rs`, `tests/icc.rs` and the plan's transcription was checked
  against ICC.1:2022-05 (profile version 4.4.0.0, the color.org PDF, read in
  the sections listed under Verified clean) and against the ICC's "How to
  interpret the sRGB color space (specified in IEC 61966-2-1) for ICC
  profiles" (color.org `chardata/rgb/sRGB.pdf`, April 2015) with the ICC
  registry's sRGB page. **IEC 61966-2-1 itself was not obtained**, it is sold
  through the IEC webstore. The ICC note is the text the code cites. The ICC
  text agrees with the code everywhere except the two places in D1 and D2.
- **AC12, independent comparison.** The reviewer wrote its own profile builder
  in Python from the ICC.1 text, not from the Rust helpers, built twelve
  matrix/TRC profiles (gamma 1.0, gamma `0x0233`, two-entry and 1024-entry
  `curv` tables, `para` types 0, 1, 2, 3 and 4, a colourant swap, a mixed TRC
  set and a wide-gamut Adobe-like set) and also took nine matrix/TRC profiles
  from the host's `/System/Library/ColorSync/Profiles` (sRGB, Adobe RGB 1998,
  Display P3, DCI P3, ITU-709, ITU-2020, ROMM, Generic RGB, ACEScg linear).
  Sixteen rows each, 8-bit and 12-bit, grey and chromatic. Ocelli through
  `IccProfile::parse`, `ColorTransform::with_icc` and `map_into`, Little CMS
  2.18 through `transicc -n -t1 -c0 -i <profile> -o '*sRGB'` and the same with
  `-s` (bounded). Results are under Evidence. **No byte differs by more than
  one code, and every one-code byte difference straddles a half code.**
  In-gamut per-channel differences reach 0.153 of 255, at near-black values
  where the 12.92 slope amplifies the 1e-4 matrix difference of D2.
  `para` types 2 and 4 differ by 5.5 and 1.1 codes at white in unbounded mode
  only, because ICC.1 10.18 clips the curve to 1.0 and Ocelli does while
  `transicc` unbounded does not. Against `-s` they are within 0.09.
- **AC6 and question 2.** Lookup-table and other refused classes were driven
  through the host's Generic CMYK, Gray and Lab profiles as well as the
  synthetic rows. They are refused, but see S1 for which name a real
  lookup-table profile gets.
- **AC10** was challenged and is unmet, D3.
- **AC11** replayed: dropping the encoding turns 7 of the 10 integration
  fixtures red, a true transpose of the colourant product turns 8 red, and
  type 3's `c` read as type 4's `e` turns the parametric fixture and two unit
  tests red. The LLD's counts are right.
- **AC13** re-executed: `bin/ocelli.sh gate wasm` green at 2,182,048 bytes
  render and 16,455 decode, no file under `ci/` changed in the range.
- Cross-story: the colour window of F-239 (D-29) consuming F-240's output was
  exercised with an out-of-gamut pixel, D1.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-pixel -p ocelli-dicom -p ocelli-render` | worktree at `d3fef64f` | now | exit 0, 576 passed, 0 failed, 83 ignored. `tests/icc.rs` 10 passed, `ocelli-pixel` lib 22 passed, `frame_open` 37 passed |
| `bin/ocelli.sh gate wasm` | worktree at `d3fef64f` | now | exit 0, render 2,182,048 (ceiling 2,258,755), decode 16,455 (ceiling 17,277), ALL GREEN |
| `cargo tree -p ocelli-wasm -e normal --target wasm32-unknown-unknown --prefix none`, and with `--no-default-features --features decode-worker` | tip | now | render reaches ocelli-cache, core, pixel, render, seg, viewport, volume. Decode reaches cache, core, pixel. Neither reaches ocelli-dicom |
| Base wasm size 2,179,981 and the twiggy attribution | base `6163b82f` | prior receipt only, not re-executed | not checked |
| `transicc` 2.18 versus Ocelli, 21 profiles by 16 rows | reviewer-built and host profiles | now | max byte difference 1, each at a half-code boundary. Table below |
| Mutation catalogue, 22 mutants plus 3 AC11 replays | scratch copy with its own `CARGO_TARGET_DIR` | now | 6 survivors, D7, D8, N3 |
| Window on an out-of-gamut pixel | host Adobe RGB 1998, device `0 255 0`, `ColourWindow::new(64, 256, false)` | now | unclipped reply `-1312.2 255.0 -141.5` windows to `0 255 0`, the clipped `0 255 0` windows to `64 255 64` |
| `git status`, `git reflog`, `ls -la /dev/null` | worktree | now | clean apart from the two setup symlinks, `/dev/null` still a character device, no reflog entry after the commit |

Per-channel maximum absolute difference from `transicc`, in codes of 255. "In
gamut" covers rows whose unbounded `transicc` reply lies in 0 to 255. "Clipped"
compares Ocelli clamped to 0 to 255 against `transicc -s` on every row.

| Profile | Rows | In gamut R G B | Clipped R G B |
|---------|------|----------------|---------------|
| gamma1 | 16 | 0.0272 0.0129 0.0126 | 0.1190 0.0276 0.0196 |
| gamma0233 | 16 | 0.0859 0.0275 0.0402 | 0.1190 0.0276 0.0402 |
| table2 | 16 | 0.0272 0.0116 0.0099 | 0.1190 0.0276 0.0196 |
| table1024 | 16 | 0.0499 0.0234 0.0222 | 0.1190 0.0276 0.0222 |
| para0 | 16 | 0.0565 0.0218 0.0371 | 0.1190 0.0276 0.0371 |
| para1 | 16 | 0.1530 0.0195 0.0242 | 0.1530 0.0417 0.0242 |
| para2 | 16 | 0.0353 0.0213 0.0097 | 0.0473 0.0548 0.0172 |
| para3_srgb | 16 | 0.0693 0.0257 0.0348 | 0.1190 0.0276 0.0348 |
| para4 | 16 | 0.0514 0.0232 0.0173 | 0.0758 0.0916 0.0274 |
| swap | 16 | 0.0181 0.0228 0.0058 | 0.1190 0.0276 0.0196 |
| mixed | 16 | 0.0550 0.0298 0.0635 | 0.1190 0.0298 0.0635 |
| adobe_gamma22 | 16 | 0.0148 0.0218 0.0193 | 0.0185 0.3764 0.1111 |
| host sRGB Profile | 16 | 0.0716 0.0199 0.0267 | 0.0716 0.2198 0.0630 |
| host AdobeRGB1998 | 16 | 0.0148 0.0218 0.0193 | 0.0185 0.3764 0.1111 |
| host Display P3 | 16 | 0.0147 0.0213 0.0231 | 0.0192 0.0421 0.0231 |
| host DCI(P3) RGB | 16 | 0.0142 0.0037 0.0048 | 0.0177 0.0775 0.0048 |
| host ITU-709 | 16 | 0.0481 0.0211 0.0210 | 0.0481 0.2198 0.0630 |
| host ITU-2020 | 16 | 0.0193 0.0289 0.0193 | 0.0193 0.0809 0.0193 |
| host ROMM RGB | 16 | 0.0156 0.0040 0.0053 | 0.0193 0.0456 0.0262 |
| host Generic RGB | 16 | 0.0935 0.0210 0.0449 | 0.2203 0.0945 0.0449 |
| host ACEScg Linear | 16 | 0.0171 0.0045 0.0058 | 0.0185 0.0251 0.0528 |

The 0.119 to 0.376 clipped figures are all on primary rows near zero, where
Little CMS's built-in sRGB (built from the primaries' chromaticities) and
Ocelli's matrix (built from IEC's four-decimal matrix) leave a different
residue of order 1e-4 below the 12.92 knee. Rounding of the published
constants explains them. The five one-code byte differences were ACEScg at
22.4988 against 22.5156, ACEScg 12-bit at 20.5349 against 20.4821, DCI P3
12-bit at 22.5514 against 22.4739, ITU-2020 at 198.4928 against 198.5080 and
ITU-709 at 198.4964 against 198.5116, every one a half-code straddle.

## Defects

### D1, an out-of-gamut colour leaves stage 4 unclipped, so the colour window and readout act on values sRGB does not have

**Where**: `crates/ocelli-pixel/src/icc.rs:334` to `:345` (`IccTransform::apply`)
and `:382` (`encode_srgb`). The doc at `:307` to `:311` says "The reply is
display codes at `0 ..= 255` as `f32`".
**What**: the matrix output is encoded and scaled by 255 with no clip, so a
device colour outside the sRGB gamut replies far outside 0 to 255. Host Adobe
RGB 1998, device `0 255 0`, replies `-1312.2031 254.9901 -141.4999`. Device
`255 0 0` replies `295.32` red. F-239's colour window then acts on those
values. With `ColourWindow::new(64, 256, false)` the unclipped reply windows
to `0 255 0`, while the clipped sRGB value `0 255 0` windows to `64 255 64`,
64 codes apart in two channels. Any window whose low end is below 0 or whose
high end is above 255 shows it, and statistics and the readout on a profiled
frame would report the raw out-of-range values.
**Why it is wrong**: the ICC's sRGB note the module cites, section A.7: "RL,
GL or BL values less than 0 or greater than 1 are clipped to 0 and 1", then
"Produces sRGB digital values with a range 0 to 1". ICC.1:2022 Annex F.3
equations F.8 to F.16 clip the linear value to 0 and 1 before the inverse
TRC on the PCS-to-device side. The registry page gives "Range: linear RGB:
0.0 to 1.0". The apply doc's own claim is false on these rows. DICOM PS3.3
C.11.15.1.2 names ADOBERGB and DISPLAYP3 as defined terms, so wide-gamut
profiles are an expected input, not a corner.
**Evidence**: `window_on_out_of_gamut` in the reviewer's scratch test, output
quoted above. `transicc -s` clips to 0 and 255 on the same rows.
**Scope and regression**: present on HEAD. Affects every consumer of
`map_into` with a profile attached: tier C bytes under a non-default window,
the readout and statistics the plan says act on converted values. The
initial window is unaffected because the byte store clamps. Smallest
regression: the AC5 or an Adobe-like fixture with a chromatic device value
asserting the reply is within 0 and 255 and that a window with low end below
0 gives the clipped answer.

### D2, the PCS to sRGB matrix is built with a white point that is not its source matrix's own, and the error it causes is recorded as s15Fixed16 quantisation

**Where**: `crates/ocelli-pixel/src/icc.rs:348` to `:376`, the test module
comment `crates/ocelli-pixel/tests/icc.rs:20` to `:26`, and
`docs/lld/pixel-pipeline.md` "The largest relative linear error the
quantisation leaves is about `2e-4`".
**What**: the comment says the Bradford adaptation used the D65 white
"`0.9505 1.0 1.0890` at `Y = 1`". Recomputing from those inputs gives
`3.133929911843798` for the first entry, not the `3.1338520414158704` the
comment lists "for the reviewer". The listed values are reproduced to 4e-16
only with the unrounded chromaticity white `0.950456 1 1.089058` against
IEC's four-decimal matrix, whose rows sum to `0.9505 1.0 1.0890`. The two
whites disagree, so the constant maps the D50 PCS white to linear
`0.99983 1.000045 1.000059`, not `1 1 1`. The ICC note's own D50 matrix
(`0.436030342570117 ...`) inverts to a matrix that the constant misses by
1.0e-4, while the self-consistent derivation misses it by 1.8e-5. On the
fixtures' neutral rows the error at 255 is `-0.0213 +0.0080 +0.0070` codes.
With the consistent white the row sums are `0.99998 1.00003 1.000004` and the
error is about 0.0025. So the "0.0213" the module comment and the LLD
attribute to "the quantisation" is about 90 percent this mismatch.
**Why it is wrong**: the plan's Approach and the ICC note section 2 define
the PCS to sRGB matrix as the inverse of the published D50 colourant matrix,
which is "this matrix combined with the inverse of A.7". A doc comment whose
stated inputs do not produce its stated outputs, and an LLD sentence that
names the wrong cause for a measured error, are false claims.
**Evidence**: Python with numpy, the seven input combinations of four-decimal
or chromaticity D65 white against three D50 encodings, only one reproducing
the constant. `code @ [0.9642, 1, 0.8249]` printed above.
**Scope and regression**: present on HEAD. Magnitude about 0.02 of a code at
white, up to 0.15 code near black on chromatic rows (the para1 row of the
`transicc` table). Smallest fix: derive with the white the matrix carries, or
invert the ICC note's D50 colourants, and re-measure the quantisation figure.

### D3, the GPU tiers are said to report a profiled frame unavailable, and no code does

**Where**: `docs/lld/feature-availability.md` added paragraph,
`docs/lld/pixel-pipeline.md` "Tier A and tier B keep `ColourLut::icc`
unavailable ... and a GPU route given a profiled frame reports unavailable
rather than drawing device RGB", repeated after the tier C paragraph.
`crates/ocelli-render/src/colour.rs:146` still documents `ColourLut::icc` as
"ICC is not implemented."
**What**: nothing in `ocelli-render` reads a profile. `git grep` finds no
caller of `ColourLut::icc` outside its own test, and `ColourLut::palette`
bakes palette texels through a `ColorTransform` that never has an
`IccTransform` attached, so a GPU caller handed a palette frame that carries
a matrix profile gets device-RGB texels and no refusal. The feature
availability paragraph also says a refused profile "keeps the CPU route's own
bytes on every tier", which no GPU route implements.
**Why it is wrong**: plan AC10, "a GPU caller of a profiled frame reports
unavailable rather than drawing device RGB", evidence `crates/ocelli-render`
tests, and CLAUDE.md's tier rule that a feature unavailable on a tier reports
so rather than producing a different result. No render test was added.
**Evidence**: `git grep -n "ColourLut::icc\|with_icc" -- crates examples
packages tools` lists only the render test of F-239 and `color.rs`.
**Scope and regression**: present on HEAD. AC10 is unmet. Either the GPU
route takes the profile presence and refuses, with the render test the plan
names, or the LLD says only what exists. The stale `ColourLut::icc` doc
should move with it.

### D4, current-state LLD prose describes callers that do not exist on this branch

**Where**: `docs/lld/pixel-pipeline.md` "A refused parse holds the refusal, so
a retry gives the same answer without a second parse" and "charged to the
held frame through `IccTransform::heap_bytes`", and `docs/lld/frame-open.md`
"The bytes go to `IccProfile::parse`, whose refusals are the frame's kind".
**What**: no code calls `IccTransform::heap_bytes`, `ColorTransform::with_icc`
or `icc_profile` outside tests, nothing holds a refusal, and nothing passes
the element's bytes to the parser. These are the viewer part's behaviours.
**Why it is wrong**: `docs/lld/pixel-pipeline.md` line 10 says it "describes
what the code does today". Microscope section 3, a false claim in prose.
**Evidence**: the same `git grep`, plus `git grep -n "heap_bytes()"` showing
only the folder viewer's unrelated `ValueFacts`.
**Scope and regression**: present on HEAD. If the viewer part lands in the
same integration these sentences become true, so the fix is either to scope
them to the viewer part or to land them with it. Re-check at integration.

### D5, the palette fixture's expected values are the implementation's output, not the hand computation it shows

**Where**: `crates/ocelli-pixel/tests/icc.rs:657` to `:722`,
`palette_entries_are_normalised_by_their_declared_depth`.
**What**: the comment computes `32767 / 65535 = 0.500008` and `16384 / 65535
= 0.250008`. They are `0.499992` and `0.250004`. Their encodings are
`187.5147` and `136.9611`, while the asserted values are `187.521` and
`136.9656`. Ocelli's own neutral-axis offsets from D2 are `+0.0060` green
and `+0.0045` blue near those levels, which is exactly the gap, so the
asserted numbers are Ocelli's replies and the intermediate figures were
back-filled to match.
**Why it is wrong**: HLD 27.2 R2 and microscope section 2, a fixture value
copied from the implementation. The 0.05 tolerance hides it today, and the
value would follow a wrong implementation that moved by less than that.
**Evidence**: Python, `32767/65535` and `1.055 * x^(1/2.4) - 0.055`, printed
above.
**Scope and regression**: present on HEAD, test-only. Replace with the hand
values `187.5147` and `136.9611` and their correct intermediates.

### D6, AC5's recorded wrong-implementation reply is false

**Where**: `crates/ocelli-pixel/tests/icc.rs:463` to `:465`, "The transposed
wrong implementation answers `214 152 106` on this row".
**What**: a true transpose of the colourant product, applied in a scratch
copy and run through `map_into` on the swapped profile, replies
`175.5268 181.2294 100.5785` on device `255 0 0`, bytes `176 181 101`. No
transposition variant of the six tried gives `214 152 106`.
**Why it is wrong**: the plan's AC5 asks the implementer to compute and record
these bytes by hand. A false recorded figure is a false claim.
**Evidence**: the scratch mutation's `reviewer_dump` output quoted above.
**Scope and regression**: test comment only. The assertion itself still
rejects the transpose.

### D7, four guards in the parser and transform survive their removal

**Where**: `crates/ocelli-pixel/src/icc.rs:679` (the s15Fixed16 sign),
`:432` (the 10.6 and 10.18 range clip), `:551` to `:553` (the `XYZ ` type
check), `:704` (`table.depth == depth`).
**What**: each mutant stayed green across `--lib`, `tests/icc.rs`,
`tests/color.rs` and `tests/palette.rs`. No fixture parses a negative
s15Fixed16, so `Ok(value)` in place of the sign passes, yet `para` type 1 and
2 parameters such as a negative `b` are the normal way to write those curves.
Removing `value.clamp(0.0, 1.0)` passes, yet the `transicc` table shows that
clip is worth 5.5 codes at white on a type 2 curve. Removing the `XYZ ` type
check passes. Removing the depth check passes, and the unit test
`tabulated_codes_match_the_continuous_curve` says "A depth the tables were not
built for falls through to the curve", but its 12-bit codes 4095 and 2048 fall
through on the table's length alone, so that block passes both ways. Cycle 2
item 5 of the self-review records rewording a fall-through claim of exactly
this kind.
**Why it is wrong**: CLAUDE.md, every function doing pixel arithmetic needs a
hand-computed fixture, and microscope sections 2 and 4, a test that passes on
the wrong code and a guard nothing executes.
**Evidence**: mutants M4, M7, M18 and M9 of the catalogue below.
**Scope and regression**: present on HEAD. Smallest regressions: one `para`
fixture with a negative parameter hand computed from Table 68, one type 2 or
4 curve whose value exceeds 1 at the top, one colourant tag of the wrong
type, and a table built for 8 bits read at 12 bits with a code below 256.

### D8, the three YCbCr routes with a profile attached have no test

**Where**: `crates/ocelli-pixel/src/color.rs`, the `YbrFull`, `YbrFull422`
and `YbrPartial422` arms of `pixel` and the YBR arm of `with_icc`.
**What**: replacing the `YbrFull` arm's `map_or(rgb, |icc| icc.apply(...))`
with the bare `rgb` stays green. Nothing exercises the 8-bit normalisation
the `with_icc` doc defines for those routes or the curve fallback for their
fractional device values.
**Why it is wrong**: PS3.3 C.11.15.1.1 says the profile's input "shall be
RGB ... regardless of the Photometric Interpretation of the image pixel data
prior to decompression", so the YBR routes are the ones that carry profiles
from JPEG ultrasound and secondary capture, the plan's own motivating files.
An untested pixel arithmetic path, CLAUDE.md hard rules.
**Evidence**: mutant M12.
**Scope and regression**: present on HEAD. One `YBR_FULL` fixture with the
gamma 1.0 profile whose expected value is hand computed through PS3.3
C.7.6.3.1.2 and then the sRGB encoding.

## Smells

### S1, a lookup-table profile above 16 KiB is named too large, not lookup-table

**Where**: `crates/ocelli-pixel/src/icc.rs:117` to `:119`, before the header
is read.
**What**: the host's 55,280-byte Generic CMYK profile is `TooLarge`. A
lookup-table profile with a `lut16Type` 33-point grid is above 200 KB, and
PS3.3 C.11.15.1.1 recommends exactly that ("All LUTs should be represented
as 16 bit values, using tag type lut16Type"), so the scanner profiles the
plan expects to name as "lookup-table" will mostly be named "too large".
**Why it is a hazard**: the operator's answer to question 3 and AC9 name the
kind on the page. The header and tag table are 132 plus 12 per tag bytes and
can be classified from any length safely before the size refusal applies to
what is actually parsed.
**Evidence**: `reviewer_dump` over the host profiles, `Generic_CMYK_Profile
REFUSED Err(TooLarge)`.
**Scope**: the viewer part will inherit it when it maps `IccError` to the
page sentence.

## Nitpicks

- N1. `DToB3Tag` (`D2B3`) is missing from `LOOKUP_TABLE_TAGS`. ICC.1:2022
  8.10.2 would not use it for the media-relative intent this transform
  applies, so no pixel changes, but the plan's item 1 refuses "any profile
  carrying ... another lookup-table tag".
- N2. Table 68 NOTE 1 leaves parameters such as `d < -b/a` or a negative `a`
  undefined. `powf` of a negative base returns NaN, `clamp` keeps NaN, and
  NaN reaches the display value. Refusing such a curve by name would be
  clearer than a NaN the byte rule turns into 0.
- N3. `IccProfile::rendering_intent` has no test and no consumer. Swapping
  values 0 and 1 stays green (M16).
- N4. `MAX_PROFILE_BYTES`'s doc and the LLD say the corpus "carries no ICC
  profile at all". The manifest has no ICC row, but `corpus/data/synthetic/
  sc_rgb_icc.dcm` from F-239 carries a 44-byte placeholder that this parser
  names `Malformed`. Worth one clause once F-239's rows land.
- N5. Unconfirmed concern. The exact size check at `:128` to `:131` refuses a
  profile whose length is odd once DICOM pads it to even length. ICC.1:2022
  7.4 pads every tag including the last, so a v4 profile is a multiple of
  four, but the v2 text (ICC.1:2001-04 and earlier) was not read to confirm
  v2 profiles carry the same rule.

## Verified clean

- ICC.1:2022 7.2 Table 17: size at 0, version at 8 (BCD `04400000h`), class
  at 12, data colour space at 16, PCS at 20, `acsp` at 36, intent at 64 as a
  `uInt32Number` with Table 23 values 0 to 3, PCS illuminant at 68 as
  `0.9642 1.0 0.8249` (the fixtures' `F6D6 10000 D32D`). 7.2.2 exact size.
  7.3.1 Table 24 count then 12-byte entries, offset from byte 0 of the header.
- Table 18 classes (`scnr`, `mntr`, `prtr`, `link`, `spac`, `abst`, `nmcl`)
  and Table 19 signatures (`RGB `, `GRAY`, `XYZ `, `Lab `).
- 8.3.3 and 8.4.3: the six matrix/TRC tags, "Only the PCSXYZ encoding can be
  used with matrix/TRC models". 8.10.2: lookup-table tags take precedence over
  TRC and colourants, which backs refusing a profile carrying both.
- 10.6 Table 35 `curveType`: count at 8, entries at 12, `n = 0` identity,
  `n = 1` a `u8Fixed8Number` gamma read as `y = x^gamma`, `n > 1` entries
  over `[0, 1]` spaced `1/(n-1)` with linear interpolation, divisor 65,535.
- 10.18 Tables 67 and 68 `parametricCurveType`: function type at 8 as
  `uInt16Number`, parameters at 12 as `s15Fixed16Number`, counts 1, 3, 4, 5,
  7, and every branch and boundary (`X >= -b/a`, `X >= d`), read from the
  rendered page because text extraction drops the operators. The range clip.
- 10.31 Table 85 `XYZType`: signature, 4 reserved, then XYZNumber triples,
  20 bytes for one.
- 4.6 Table 4 `s15Fixed16Number` two's complement over 65,536 including
  `80000000h`. 4.9 Table 7 `u8Fixed8Number` over 256. `u16Fixed16Number`
  (4.7) is not used by the code.
- Annex E.3 Equation E.1 linearised Bradford constants, as the inputs the
  reviewer used to reproduce the constant in D2.
- Annex F.3 Equation F.6: colourants are columns, `product` puts each tag in
  a column, confirmed by AC5 and by the swap row against `transicc`.
- The sRGB encoding knee `0.0031308`, slope 12.92, `1.055 L^(1/2.4) - 0.055`,
  against the ICC note A.8 and the registry page.
- The fixture expected values of AC1 (`38.2513 137.2066 187.845 224.9999`),
  AC2 (`61.773 129.0303 193.4013`, wrong gamma 2 gives `71 137 198`), AC3
  (sRGB parameters encode to `26666h F2A7h 0D59h 13D0h 0A5Bh`, wrong branch
  `5.71` and `2.75`), AC4, AC8 (`187.5365`) and the R1 palette (`187.845`)
  all reproduce by hand.
- No `as` cast and no `#[allow]` is added anywhere in the range. Every
  conversion is `from`, `try_from` or an exact integer test. The
  s15Fixed16 split is exact to 2^24 as its comment says.
- `icc_profile` returns `None` for absent, empty and word-projected elements
  and the test goes red when it accepts words (M22).
- Mutation catalogue, all in the scratch copy with its own target directory.
  Red: M1 drop encoding, M2 zero product, M3 type 3 `c` as `e`, M5 one PCS
  entry off by 1e-3, M6 knee at 0.04045, M8 `D2B2` dropped, M10 no
  interpolation, M11 gamma over 255, M13 palette depths forced to 8, M14
  tables over `2^n`, M15 RGB route on scaled samples, M17 size check off,
  M19 type 1 low branch, M20 tag bound check off, M21 type 4 `f` dropped,
  M22 `icc_profile` accepts words. Green: M4, M7, M9, M12, M16, M18.
- wasm: gate green, render 2,182,048 matches the LLD and the caller's figure,
  decode 16,455 unchanged, no budget file edited, `cargo tree` claims hold.
- use.html: the crate-table cell and the stage 4 sentence are true of the
  library. README and more.html are untouched and say nothing the diff makes
  stale. No em-dash, en-dash or prose semicolon in any added line.
- Worktree: only the commit changed tracked content. The `git checkout HEAD
  -- /dev/null` fragment GLM disclosed left no trace, `/dev/null` is still a
  character device, and the reflog shows nothing after the commit.

## Self-review catch figure

Of the 14 findings, 9 were within reach of the method
`.claude/scratch/F-240-self-review-1.md` says it applied ("every sentence
executed, every test broken in scratch where a break is meaningful, every
number re-measured"): D1 (the note it names states the clip), D2 (its
comment hands the reviewer f64 values its stated inputs do not produce), D3
and D4 (sentences that cannot be executed), D5 and D6 (numbers it says it
re-measured), D7 (cycle 2 item 5 reworded this exact fall-through claim and
the block still passes both ways), D8 (a mutation of the route it wrote) and
N3. The other 5, S1, N1, N2, N4 and N5, need knowledge outside the brief.
All 8 defects are among the 9.

## Limitations and handoff

- IEC 61966-2-1 was not read. The ICC's interpretation note and registry
  page stood in for it, as the code's own comments do.
- The ICC v2 text was not read, so N5 stays unconfirmed.
- The base wasm size and the twiggy symbol attribution are prior receipts.
- The folder viewer, corpus rows, benchmarks and AC9 and AC14 to AC16 are
  the viewer part's and were not in this commit.
- No GPU was used. D3 is a reading of the code and a `git grep`, not a GPU
  run.
- The reviewer's scratch copy, profiles and spec downloads were deleted with
  Python `shutil` after the pass. Host profiles were read in place and copied
  only into that scratch directory.
- Next owner: the implementer remediates D1 to D8 and S1, then a fresh pass 2
  reviews the changed material.
