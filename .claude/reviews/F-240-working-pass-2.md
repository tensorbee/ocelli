# F-240 review, pass 2

**Reviewed**: `work/f-240-xai` range `1c917d3b..72a75882` at tip `72a75882`,
the single commit "F-240, review pass 1 remediation in the library". No
unstaged tracked path. The only untracked paths are the `.venv` and
`tools/oracle/out` setup symlinks.
**Author / reviewer**: remediation by GLM in opencode, with self-review notes
`.claude/scratch/F-240-self-review-1.md` to `-4.md`. Reviewed by Claude
(Opus) as an independent `/microscope --working` pass. The reviewer wrote
none of the reviewed material and did not write pass 1.
**Result**: 6 defects, 0 smells, 6 nitpicks

## Pass 1 findings replayed

| Finding | Replay through the real caller | State |
|---------|--------------------------------|-------|
| D1 gamut clip | `IccTransform::apply` clamps each linear channel to 0 and 1 before `encode_srgb`. Adobe green through `IccProfile::parse`, `with_icc`, `map_into` gives `0 254.9859 0`, the windowed bytes `64 255 64`. Removing the clamp turns 2 tests red, removing only its upper half turns 1 red | closed |
| D2 matrix source | `PCS_TO_SRGB` is the `f32` rounding of the inverse of the note's printed D50 matrix, all nine entries bit-equal (numpy, independent of the unit test). The printed matrix was re-read in the note itself, `color.org/chardata/rgb/sRGB.pdf`. D50 maps to `0.99999999 1.00000003 0.99999989`. The comment's `f64` values agree with the inverse to 7e-16. A 1e-4 change of one entry turns the unit test red | closed, but see new D3 on the quotation |
| D3 GPU refusal | `ColourLut::palette` takes `IccProfilePresence` and refuses `Accepted` before the bake. The only GPU colour routes are preset (display keyed, monochrome) and palette, so no other route bakes device RGB. Removing the guard turns `a_profiled_palette_is_not_baked_as_device_rgb` red, and that test runs on the floor (not ignored) | closed |
| D4 LLD viewer-part claims | `frame-open.md` and `pixel-pipeline.md` now scope the recipe, refusal holding and charging to the viewer part | closed for the LLDs. A same-class sentence survives in `color.rs:660`, N4 below |
| D5 palette hand values | The comment now derives `187.5147` and `136.9611` correctly from `32767/65535 = 0.4999924` and `16384/65535 = 0.2500038` | closed, but the tolerance departs from the ruling, new D2 |
| D6 transposed reply | Recomputed: `0.4316441 0.4633521 0.1289888`, encoded `175.5416 181.2241 100.5762`, bytes `176 181 101`. Matches | closed |
| D7 guards | Sign drop (M5), range clip (M6), `XYZ ` check (M7) and table depth check (M8) each turn at least one test red | closed |
| D8 YBR routes | Each of the three arms with the profile dropped turns its own fixture red (M9 to M11). Every expected value reproduces from the PS3.3 C.7.6.3.1.2 forward matrices inverted in numpy | closed |
| S1 classify before size | A 16,384-byte-plus `A2B0` profile is `LookupTableProfile`. Moving the size check back to the top turns 2 tests red (M12) | closed |
| N1 `D2B3` | In the list and the loop. Dropping it turns the refusal test red (M14) | closed |

## Acceptance challenged

- **AC12 arithmetic, independently derived.** Every fixture value in the
  changed tests was recomputed in Python from ICC.1:2022 Table 4 (s15Fixed16
  over 65,536), 10.6, 10.18 Table 68, Annex F.3 (colourants as columns), the
  note's A.7 clip and A.8 encoding, and PS3.3 C.7.6.3.1.2. All reproduce: the
  D1 rows (`-0.3981993`, `-0.0429451`, `0.9998680` to `254.9852`, the
  unclipped `-1311.9`), the type 1 negative-`b` rows (`70.4356`, `0.0127`,
  two exact zeros), the type 2 clip row (`-0.2583682 0.9998803 0.0613573`,
  `254.9866 70.0635`, unclipped blue `67.5591`), the three YBR rows and the
  `+ 16` omission (`247.0932 214.7475 183.6771`), and the Adobe colourant
  encodings, which also equal the host `AdobeRGB1998.icc` tags byte for byte.
- **The palette tolerance**, item 3 of the brief, below under D2.
- **The quantisation figures.** `0.0436` is the unit-cube worst case, at
  device `1 0 1`, green, reproduced as `0.043585`. `0.0054` is the Adobe
  red-row green channel, `0.005377` measured, and its attribution to the
  Adobe colourants' own quantisation is right (`0.348069` decimal against
  `0.353342` quantised). `2.4e-5` and `1.4e-5` do not reproduce, D4.
- **AC13** re-executed, D1.
- **Tiers.** The sRGB encoding exists once: `git grep` for `0.0031308`,
  `003_130_8`, `12.92` and `1.055` outside `icc.rs` and its test finds
  nothing in `crates`, `packages`, the oracle sources or the folder viewer.
  Tier C reaches the transform only through `ColorTransform::map_into` in
  `colour_bytes_into`. Tiers A and B have no ICC arithmetic and refuse.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-pixel -p ocelli-render --test icc --test colour --lib` | worktree at `72a75882` | now | exit 0. `tests/icc.rs` 16 passed, pixel lib 24, render `colour` 9 passed 2 ignored (GPU) |
| `bin/ocelli.sh gate wasm` | worktree at `72a75882` | now | ALL GREEN, render 2,182,144 (baseline 2,151,196, ceiling 2,258,755), decode 16,455 |
| `git diff --stat 1c917d3b..72a75882 -- ci/` | range | now | empty, no budget file edited |
| Mutation catalogue M1 to M23 | `git archive` of the tip in scratch, own target dir | now | every mutant red, listed under Verified clean. M23 survives the palette fixture as written, D2 |
| Fixture dump | scratch copy with `assert_close` printing actual and expected | now | per-row deltas quoted in D2 and D4 |
| numpy over the printed matrix, the s15 encodings and an `f32` emulation of `product` | the note's nine values | now | quoted in D2 to D4 |
| pydicom scan of `corpus/data` | 113 readable files | now | one file with `(0028,2000)`, `synthetic/sc_rgb_icc.dcm`, 44 bytes, as the doc comment says |
| `python3 scripts/prose_check.py` | worktree | now | clean over 912 files. It does not scan `.rs`, D6 |

## Defects

### D1, the build-targets LLD records a tip size the tip no longer has

**Where**: `docs/lld/build-targets.md:329` to `:331`.
**What**: "On its tip they are 2,182,048 and 16,455: the render module grew
by 2,067 bytes". The tip at `72a75882` measures 2,182,144, so the growth
over base 2,179,981 is 2,163 bytes. The remediation added 96 bytes, most
plausibly the three linear clamps in `apply`, and the file was not touched.
GLM's own `.claude/scratch/F-240-self-review-4.md:27` records 2,182,144.
**Why it is wrong**: plan AC13, "Recorded in `docs/lld/build-targets.md`",
and microscope section 3. A size figure that is stale the moment it lands.
**Evidence**: `bin/ocelli.sh gate wasm` output above.
**Scope and regression**: doc only. Update the tip figure, the delta and the
percentage, and add the 96 bytes to the attribution or say it was not
re-attributed.

### D2, the palette fixture widens a ruled tolerance that did not need widening, and loses a mutant for it

**Where**: `crates/ocelli-pixel/tests/icc.rs:896` (`PALETTE_TOLERANCE = 1e-2`)
and `:963`.
**What**: the lead ruled the palette fixture "asserted within 1e-3". GLM
kept the unquantised hand value `187.5147` and widened to 1e-2. The
quantisation figure GLM gives is right: with the colourants as the profile
actually carries them (each s15Fixed16 word over 65,536, ICC.1 Table 4) the
hand value is `254.99969 187.51686 136.96207`, `0.0021061` above the
unquantised green. But that quantised value is itself a hand computation
from the specification, not the implementation's output, and the pipeline
replies `254.99971 187.51683 136.96205`, within 3e-5 of it. So a 1e-3 test
is possible with a margin of about thirty, and the deviation is unnecessary.
It also costs coverage: M23, normalising every channel by `2^n` rather than
`2^n - 1`, moves the 16-bit palette green by about 0.0013 and stays green
under the 1e-2 fixture, while the same fixture with 1e-3 and the quantised
values turns red. (8-bit fixtures catch M23 elsewhere, but this is the only
16-bit palette row.)
**Why it is wrong**: the lead ruling on D5 and D6, and WORKFLOW.md "A
tolerance needs widening to pass: Stop. This is a design-plan decision, not
a fix". Disclosing the change does not stand in for a ruling.
**Evidence**: the fixture dump and the M23 rerun with the variant fixture,
both in scratch.
**Scope and regression**: test only. Assert `(254.9997, 187.5169, 136.9621)`
within 1e-3, with the comment deriving them from the s15 words of `R_XYZ`,
`G_XYZ` and `B_XYZ` times the printed inverse, keeping `187.5147` as the
unquantised reference. If the lead would rather keep the unquantised value,
that is a ruling change to record, not an implementer choice.

### D3, the note is quoted as saying the opposite direction from what it says

**Where**: `crates/ocelli-pixel/src/icc.rs:378` to `:381`, the unit test doc
`:785` to `:788`, and `docs/lld/pixel-pipeline.md:553` to `:556`.
**What**: each says the note "states the PCS to device direction as 'this
matrix combined with the inverse of A.7'". The note's text is "Combining
this matrix [the D65 to D50 Bradford matrix] with the inverse of the matrix
from XN, YN, ZN to RL, GL, BL in A.7 results in the following matrix, which
can be used for ICC Matrix/TRC sRGB profiles", and the matrix it then
prints is the device-to-PCS one, the very colourants the code inverts. The
note gives no PCS-to-device matrix. The constant is still right, because
ICC.1:2022 Annex F.3 makes the PCS the colourant matrix times linear RGB, so
the inverse goes the other way. Only the attribution is wrong.
**Why it is wrong**: microscope section 3, a quotation pinned to a claim the
source does not make. Pass 1's D2 used the same phrase loosely, which is
probably where it came from.
**Evidence**: the note's page 3 text, extracted from the PDF.
**Scope and regression**: prose in three places. Say that the note derives
and prints the device-to-PCS D50 matrix by that combination, and that the
PCS-to-sRGB matrix is its inverse by Annex F.3.

### D4, two figures for the quantisation residue do not reproduce, and they disagree with each other

**Where**: `crates/ocelli-pixel/tests/icc.rs:22` to `:23` and
`docs/lld/pixel-pipeline.md:563` ("largest relative linear error ... about
`2.4e-5`"), and `tests/icc.rs:890` to `:891` ("deviates from the identity by
up to about `1.4e-5` linear" for "its product with a device triple").
**What**: from the `f32` constant times the s15 colourants, emulated in
`f32`: the largest matrix entry deviation is `1.34e-5`. The largest
`|Mv - v|` over the unit cube is `2.66e-5`, at white, green. A per-channel
relative error grows without bound as that channel goes to 0, already
`2.4e-4` on a tenth-step grid. No definition gives `2.4e-5`. The `1.4e-5` is
the per-entry figure presented as the per-triple one, which is twice that.
**Why it is wrong**: microscope section 3.
**Evidence**: `f32.py` in scratch, output quoted.
**Scope and regression**: prose. Give one figure with its definition, for
example "each entry of the product is within `1.34e-5` of the identity, and
a device triple within `2.7e-5`", or delete the figure and keep the
measured 255-scale deltas, which are right.

### D5, the out-of-gamut test says 64 and 0 are one code apart

**Where**: `crates/ocelli-pixel/tests/icc.rs:520` to `:523`.
**What**: "a stored 0 windows to 64 while an unclipped -1311.9 would window
to 0, one code apart". The two windowed values are 64 codes apart. What the
sentence seems to mean is that 0 and -1311.9 store as the same byte but
window 64 codes apart.
**Why it is wrong**: microscope section 3, a false figure in a test comment,
the class pass 1 counted as D6.
**Evidence**: the LINEAR window at centre 64 and width 256 maps 0 to 64.0
and -1311.9 to 0, by PS3.3 C.11.2.1.2 by hand and by the assertion
`[64, 255, 64]`.
**Scope and regression**: one comment line.

### D6, prose semicolons in the added doc comments

**Where**: `crates/ocelli-pixel/src/icc.rs:1025`, and
`crates/ocelli-pixel/tests/icc.rs:818`, `:881`, `:883`, `:1067`, `:1068`,
`:1089`, `:1090`, `:1119`, `:1120`.
**What**: ten added comment lines join clauses with a semicolon, such as the one between "on the length alone" and
"the last is below it" at `icc.rs:1025`.
**Why it is wrong**: CLAUDE.md hard rules, "No em-dash, no prose semicolon
in tracked prose". `prose_check.py` does not scan `.rs`, so the green check
self-review 4 cites does not cover these lines. The whole crate tree at the
base held three such lines, so the rule is kept in comments.
**Evidence**: `git diff 1c917d3b..72a75882 -- crates | grep` for `; ` in
added comment lines.
**Scope and regression**: prose. Use full stops or commas.

## Smells

None.

## Nitpicks

- N1. The D1 fixture asserts `0.3481` and `0.1255` for device `255 0 0`,
  and the comment derives only the red channel. Both reproduce from the
  decimal colourants, `0.348069` and `0.125472`. Add the two products.
- N2. "at most `0.0027` of 255 on the sRGB colourant rows" holds for the
  colourant contribution, `0.002609` at AC1's 192. The type 3 parametric
  rows reach `0.002853` because the curve parameters are quantised too.
  Worth a clause.
- N3. Carried from pass 1 N3. `rendering_intent` still has no test.
- N4. `crates/ocelli-pixel/src/color.rs:660` to `:661`, "A frame whose
  profile was refused never reaches this: the recipe keeps the refusal",
  describes a viewer-part recipe that does not exist on this branch. Same
  class as pass 1 D4, which the ruling scoped to the LLDs.
- N5. Carried from pass 1 N2. Table 68 parameters NOTE 1 leaves undefined
  still produce NaN, which the byte rule stores as 0.
- N6. Carried from pass 1 N5, unconfirmed. The v2 profile padding rule was
  not read.

## Verified clean

- Mutation catalogue, scratch copy, own target directory. Each mutant is
  red. M1 drop encoding, 13 of 16 integration fixtures red, the 3 green being
  the two refusals-only rows and the colourant swap, exactly as the LLD now
  says. M2 true transpose, 14 of 16 red, as the LLD says. M3 type 3 `c` as
  `e`, the parametric fixture plus two unit tests. M4 and M4b gamut clip.
  M5 s15 sign. M6 curve range clip. M7 `XYZ ` check. M8 table depth. M9 to
  M11 the three YBR arms. M12 and M13 size check placement and removal. M14
  `D2B3`. M15 the GPU guard. M16 one matrix entry off by 1e-4, caught by the
  unit test's bit comparison only. M17 knee at `0.04045`. M18 type 1 low
  branch. M19 type 2 boundary sign. M20 s15 divisor 65,535, unit test only.
  M21 16-bit palette depth on an 8-bit palette. M22 and M23 `2^n`
  normalisation. M23 is red under the existing suite, but not through the
  palette fixture, D2.
- The new unit tests: `the_pcs_to_srgb_matrix_is_the_printed_d50_inverse`
  matches numpy's inverse bit for bit. The `s15_fixed16` encodings
  `0x00020000`, `0xFFFF8000`, `0x00026666`, `0xFFFD999A` and `0x80000000`
  decode to `2`, `-0.5`, `2.3999939`, `-2.3999939` and `-32768`. The size
  test's second half, a wrong size field at that length, is `Malformed`.
- The parse still bounds the tag-table allocation by input length
  (`table_end <= bytes.len()` before `Vec::with_capacity(count)`), so moving
  the size check below the tag table opens no unbounded allocation.
- Casts: one `as f32` was added, in the unit test, under a site-justified
  `#[allow(clippy::cast_possible_truncation)]`. `f64` to `f32` `as` rounds to
  nearest, ties to even, which is the rounding the constant claims. Stable
  `std` has no `From` or `TryFrom` for that conversion, as the comment says.
  No cast in library code.
- The D1 window test against the D-29 LINEAR window, by hand.
- `MAX_PROFILE_BYTES` doc and the LLD's corpus sentence, by the pydicom scan.
- The `IccProfilePresence` docs and the feature-availability paragraph
  against the code. Each GPU route that exists refuses or has no profile
  input.
- Wasm: decode module byte-identical, render inside the ceiling, budget
  files untouched. The figure is right in GLM's notes and wrong in the LLD,
  D1.
- No em-dash or en-dash in any added line. `prose_check.py` clean over its
  scope.
- README, `use.html` and `more.html` are not in the range. `use.html:638`
  stays true of the library.

## Limitations and handoff

- IEC 61966-2-1 itself and the ICC v2 text were not read. The note was
  fetched and read for its matrix, its A.7 clip and its derivation sentence.
- No GPU was used. The two ignored render tests were not run. D3 of pass 1
  was closed by code reading, `git grep` and the floor test.
- `transicc` was not re-run. The matrix change moves results by at most
  1e-4 linear against pass 1's table, which was within one code everywhere.
- The base wasm size, 2,179,981, is a prior receipt.
- Acceptance rows still owed, none closed by this commit: AC9 (folder
  viewer and browser test), AC14, AC15 and AC16 (benchmarks, external),
  AC13's recording (D1 here), and the viewer part's `ocelli-dicom`,
  `inspect_json`, corpus rows and `README.md` work. AC12's `transicc` record
  in `docs/lld/pixel-pipeline.md` should be re-measured against the new
  matrix when the viewer part lands.
- Scratch copy, profiles and the note's PDF were deleted with Python
  `shutil` after the pass.
- Next owner: the implementer fixes D1 to D6, then a fresh pass 3 reviews
  the changed material.

## Self-review catch figure

All 6 defects were within reach of the method self-review 4 says it
applied. D1 is a figure GLM measured itself and did not carry into the LLD.
D2 is a ruling it knowingly departed from. D3 needs one reading of the page
the comment cites. D4 is a figure "re-measured" that no measurement gives.
D5 is a sentence next to its own assertion. D6 is a hard rule it claims to
have checked with a tool that does not scan `.rs`. Of the 6 nitpicks, N1,
N2 and N4 were also within reach. **The self-review should have caught 9
of 12**, all 6 defects among them.
