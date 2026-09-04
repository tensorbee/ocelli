# F-009, implementation review, pass 6

**Reviewer**: independent agent, wrote neither the work nor pass 1, 2, 3, 4 or 5
**Diff reviewed**: working tree, base cd74768, branch `work/f-009-claude`
**Result**: 1 defect, 1 smell, 3 nitpicks

The hard rule first. A Python walk over every file in the worktree outside
`.git`, ignored trees included, reading bytes 128 to 131 of each of **1,609**
files and separately matching `.dcm`, `.dicom`, `.ima`, `.img` and `.dc3` by
suffix, case insensitively: **zero hits on both**. No DICOM is in this
repository. All 91 manifest rows resolve and verify under `$OCELLI_CORPUS_DIR`.

Twenty mutations were applied, every one reverted from a copy taken first and
every revert proved with `diff -q`. At the end of this pass `diff -rq` against
a copy of the whole worktree taken before it started reports one difference,
`target/.rustc_info.json`, which `bin/ocelli.sh gate --floor` rewrote. No
tracked or staged file differs, the two ignored `__pycache__` trees were
restored to the bytes they had, and `diff -rq` over the corpus reports nothing
at all. Everything below was run with `PYTHONDONTWRITEBYTECODE=1` and a cleared
cache, per the note pass 5 left.

---

## Round 6's changes, verified

Each by mutation, not by reading. All four hold.

### `ToolchainPins` membership. Closed, in both directions

Pass 5's three green mutations are now red, and so is the direction it named
but did not write.

| # | Mutation | Result |
|---|----------|--------|
| M4 | `ci.yml` `"pylibjpeg-openjpeg==2.5.0"` to `"pylibjpeg-openjpeg"` | **RED**, `test_ci_pins_agree_with_built_with` |
| M4b | `ci.yml` `"pyjpegls==1.5.1"` to `"pyjpegls"` | **RED**, same test |
| M7 | `ci.yml` `apt-get install -y dcmtk` to `dcmtk=3.6.7-9.1build4` | **RED**, `test_ci_does_not_pin_dcmtk` |
| M8 | a `BUILT_WITH` entry CI never installs | **RED**, `test_ci_pins_agree_with_built_with` |
| M20 | `NOT_PIP` gains `pydicom`, the escape hatch used to hide a real pin | **RED**, same test |

M20 matters because `NOT_PIP` is the one hand-written list in the class. It is
not a free pass: adding a name to it while the pin is still in `ci.yml` breaks
the set equality, so the hatch only opens for an entry CI genuinely does not
install with pip.

### The DCMTK bare-token check. Real

`re.search(r"apt-get install[^\n]*", ...)` then
`assertIn("dcmtk", install.group(0).split())`. `ci.yml` contains exactly one
`apt-get install` line, so the search cannot land on a neighbour. M7 above is
the apt pin someone would actually write and it goes red. M9, `dcmtk` written
`dcmtk-tools`, also goes red, so the token is bare rather than a substring.

### `coverage()`'s unknown-transfer-syntax arm. Has a test that binds

| # | Mutation | Result |
|---|----------|--------|
| M10 | `unknown = sorted(present - set(REGISTRY_TRANSFER_SYNTAXES))` to `unknown = []` | **RED**, `test_a_transfer_syntax_outside_the_registry_is_named` |
| M18 | the arm's problem message blanked, `problems.append` left in place | **RED**, the same test, and `gate corpus` |

M18 is the mutation pass 3 used to show three sibling assertions were vacuous.
This one is keyed on `"not in the registry"` and on the UID, neither of which
appears in an always-printed line, so it survives that attack.

`test_corpus_check.py` is now 17 tests, one more than pass 5 saw.

### The lossy declarations, both directions

| # | Mutation | Result |
|---|----------|--------|
| M11 | `mark_lossy` dropped on the HTJ2K lossy case | **RED**, `test_lossy_cases_declare_their_lossiness`, case `htj2k_lossy.dcm` |
| M11b | `mark_lossy` also applied to `jpegls_lossless` | **RED**, the same test, case `jpegls_lossless.dcm` |
| M21 | `.203` removed from `LOSSY_TRANSFER_SYNTAXES` | **RED**, that test and the lossless round-trip test |

Pass 5's G7 is now red and the absence half binds too. Read out of the files
rather than the code, all five lossy cases carry `LossyImageCompression` `01`
with `ISO_10918_1`, `ISO_10918_1`, `ISO_14495_1`, `ISO_15444_1` and
`ISO_15444_15`, and the eleven non-lossy cases carry neither attribute. Each
term is the right Defined Term for its algorithm under PS3.3 C.7.6.1.1.5.1.
What is still unasserted is which term goes with which case, which is S1.

`test_corpus_synth.py` is now 38 tests, one more than pass 5 saw. Fifty five in
all.

---

## The lossy thresholds, measured independently

Measured from the shipped corpus files with my own script, reading
`ds.pixel_array` against `SYNTAX_REFERENCE`'s reference case and dividing by
`(1 << BitsStored) - 1`. I checked the divisor is the right one first: the
mono16 base ramps 0 to 65535, the mono12 base 0 to 4095 and the RGB base 0 to
255, so full scale is reached in every case and no relative error is flattered
by a container wider than the content.

| Case | claimed max/full | measured max/full | measured max, in levels |
|---|---|---|---|
| `jpeg_baseline_rgb8`, 4:2:2 at q90 | 0.0157 | **0.015686** | 4 of 255 |
| `jpeg_extended_12` at q90 | 0.00049 | **0.000488** | 2 of 4095 |
| `j2k_lossy` at ratio 20 | 0.00003 | **0.000031** | 2 of 65535 |
| `htj2k_lossy` at qstep 0.001 | 0.00063 | **0.000626** | 41 of 65535 |

**All four reproduce.** So does the fifth number in the comment, the 0.0196
"transposed variant of the same shape". Reading that as the ramp's two
gradients exchanged with the 64 by 96 frame kept, which is what "of the same
shape" says, `dcmcjpeg --encode-baseline --quality 90` gives max 5 of 255,
`0.019608`. Transposing the image instead gives 0.011765, so the comment means
the first reading.

**Neither threshold is fitted to what passes.** A fitted bound would sit just
above the measurement, at 0.016 and 0.0007. Both sit above it with headroom,
0.03 against a worst measured 0.0196 and 0.005 against a worst measured
0.00063. Both are set from encoder output and the output is quoted in the
comment, which is the form that lets a reader compute the factor for himself.

**Both mutations the brief names go red.**

| # | Mutation | Result |
|---|----------|--------|
| M1 | `J2K_COMPRESSION_RATIO` 20.0 to 3000.0 | **RED**, case `j2k_lossy.dcm` |
| M2 | `JPEG_QUALITY` 90 to 1 | **RED**, cases `jpeg_baseline_rgb8.dcm` and `jpeg_extended_12.dcm` |
| M22 | `HTJ2K_QSTEP` 0.001 to 0.1 | **RED**, case `htj2k_lossy.dcm` |
| M12 | the branch removed, so the YBR case takes `TRANSFORM_MAX` | **RED**, case `jpeg_baseline_rgb8.dcm` |

M12 was written to check the branch is load-bearing rather than decorative. It
is. M22 was written to check the HTJ2K case reaches the assertion at all, since
M16 below leaves it green. It does.

Where the bounds actually bind, measured by sweeping each encoder setting and
computing the same statistic:

```text
jpeg baseline rgb8, bound 0.03    q90 0.0157  q70 0.0196  q60 0.0275  q50 0.0314
jpeg extended 12,   bound 0.005   q90 0.00049 q30 0.00122 q10 0.00366 q5 0.00586
j2k lossy,          bound 0.005   r20 0.00003 r50 0.00229 r100 0.13416
htj2k lossy,        bound 0.005   qs0.001 0.00063  qs0.01 0.00331  qs0.02 0.01239
```

See N1 for what follows from the third and fourth rows.

---

## New defects

### D1, the chroma bound's comment names the wrong cause, and it is the smaller of the two the HLD names

**Where**: `scripts/tests/test_corpus_synth.py:198-201`.

```python
# 4:2:2 discards half the chroma resolution by design, so the baseline RGB case
# is an order looser than the rest: DCMTK at quality 90 gives 0.0157 on the
# corpus content and 0.0196 on a transposed variant of the same shape.
CHROMA_SUBSAMPLED_MAX = 0.03
```

**What**: the "so" is a causal claim, and it is wrong. With chroma subsampling
entirely off the case is still an order looser than the rest. I encoded the
same `reference_rgb8.dcm` at the same quality 90 with only the sampling flag
changed, and separately with the colour transform off, and decoded each through
the same path the test uses.

**Why it is wrong**: microscope class 3, a factual sentence in a doc comment
that is checkable. And the project's own normative source disagrees with it.
HLD 25.1, which this test's docstring cites two lines below, gives the reason
for tolerance class two as "chroma subsampling **and YBR conversion**
legitimately differ". `scripts/corpus_check.py`'s `CHROMA_NOTE` quotes both
halves as well. This comment names one half, and measurement says it is the
quarter rather than the half.

The generator's own `rgb_ramp` docstring says the opposite of this comment, at
`scripts/corpus_synth.py:261-262`: "Smooth horizontally so that 4:2:2 chroma
subsampling, which averages horizontally, costs almost nothing." That is the
accurate sentence. The two are in the same diff.

**Evidence**, four controlled encodings plus an 8-bit monochrome control, all
`dcmcjpeg` 3.7.0 at quality 90 with nothing else changed:

```text
+cr   RGB colour space, no YBR at all      max 1 of 255   0.003922
+s4   4:4:4, YBR, no chroma subsampling    max 3 of 255   0.011765
      as shipped, default 4:2:2            max 4 of 255   0.015686
+np   4:1:1, twice the subsampling         max 4 of 255   0.015686
      8-bit MONOCHROME2 control, no colour max 1 of 255   0.003922
```

Decomposed: the 8-bit DCT costs 1 level, the YBR round trip costs 2 more, and
4:2:2 costs the last 1. At 4:4:4 the case measures 0.0118, which is 19 times
`TRANSFORM_MAX`'s stated basis of 0.00063 and 24 times the worst of the three
transform cases, so it still needs its own class and it still could not use
0.005. Even the 8-bit monochrome control at 0.0039 is 6 times that basis. The
looser bound is forced by 8-bit full scale and the YBR round trip. 4:2:2 is the
last quarter of it.

The chroma subsampling this comment describes is real and can dominate, on
content this corpus does not contain. Alternating one-pixel red and green
columns gives 0.647 at 4:2:2 against 0.0196 at 4:4:4. On the smooth ramp the
corpus actually ships, it does not.

**Why it matters rather than being tidy**: the constant is named
`CHROMA_SUBSAMPLED_MAX` and the branch that selects it keys on
`ds.PhotometricInterpretation.startswith("YBR")`, which is the YBR condition
and not the subsampling condition. The selector is right and the name and the
comment describe something else, so the next person to touch this reads a
rationale that points away from the code. Someone moving the case to 4:4:4 on
the strength of this sentence would expect to drop to `TRANSFORM_MAX` and would
be wrong by a factor of two.

**What is not wrong**: the value 0.03 is safe, both quoted measurements
reproduce exactly, and no behaviour changes. This is a sentence and a name.

**Fix**: say what the measurement says. "An 8-bit container and the YBR round
trip cost four levels of 255 between them, so the baseline RGB case is an order
looser than the rest", with the same two numbers after it. Renaming the
constant to match the selector, something like `YBR_EIGHT_BIT_MAX`, would close
it in the other direction too.

---

## New smells

### S1, `LossyImageCompressionMethod` is asserted by shape, so a case can name the wrong algorithm and stay green

**Where**: `scripts/tests/test_corpus_synth.py:449-462`.

```python
                    self.assertRegex(ds.LossyImageCompressionMethod,
                                     r"^ISO_[0-9]+_[0-9]+$")
```

**What**: the presence of both attributes is asserted in both directions, which
is what this round set out to do, and `LossyImageCompression` is asserted
exactly as `"01"`. The value that carries the algorithm identity is asserted
only as a shape, and every Defined Term in PS3.3 C.7.6.1.1.5.1 matches that
shape, so any of them satisfies any case.

**Evidence**, mutation M13:

```text
M13  corpus_synth.py  mark_lossy(ds, "ISO_15444_1")  ->  mark_lossy(ds, "ISO_14495_1")
     on the .91 case, so j2k_lossy.dcm declares JPEG-LS compression
     55 tests ... OK                                        <- GREEN
```

`j2k_lossy.dcm` then carries a JPEG 2000 irreversible codestream under a header
declaring `ISO_14495_1`, and the whole suite passes. That is the corpus's
declared defect class, a header claim contradicting the bytes, on the one
attribute in this pair that says which algorithm ran.

**Why it is a smell and not a defect**: the corpus is right today. I read all
sixteen syntax cases and every term is the correct one for its transfer syntax
under PS3.3 C.7.6.1.1.5.1. What is absent is anything that would notice if one
moved.

**Why it is worth a finding rather than a nitpick**: the generator's own
comment at `scripts/corpus_synth.py:735-739` says of `ISO_15444_15` that it was
"Checked against the standard, not inferred from the shape of its siblings".
The value clearly matters to the author. The test added in the same round
checks exactly the shape of its siblings.

**Fix**: a `{syntax: term}` dict beside `LOSSY_TRANSFER_SYNTAXES`, written out
in the test file rather than imported, and `assertEqual` against it. Four
entries, and the two JPEG cases share one.

---

## Nitpicks

**N1, `TRANSFORM_MAX` carries eight times the headroom `CHROMA_SUBSAMPLED_MAX`
does, and the comment does not say so.** The chroma bound is 1.5 times its
worst measurement, the transform bound is 8 times its worst, and the comment
for the second says only "The worst of those is the basis". The consequence,
measured rather than argued:

```text
M16  HTJ2K_QSTEP 0.001 -> 0.01, a ten times quality regression
     55 tests ... OK                                        <- GREEN
M17  JPEG_QUALITY 90 -> 10
     RED on jpeg_baseline_rgb8 only. jpeg_extended_12 at 0.00366 passes
```

So a tenfold HTJ2K quality drop is invisible, and a nine-tenths JPEG quality
drop is caught by the chroma case rather than by the 12-bit one. Both are
inside what the docstring promises, which is "a sanity check that the case is
the image it claims to be and not a different one" and explicitly not the
tolerance policy, and the two mutations that matter most, M1 and M2, both go
red. This is a nitpick rather than a smell for that reason. One clause saying
the bound is the worst measurement rounded up an order would stop the next
reader recomputing the factor, as I did.

**N2, a comment was orphaned by the insertion.** `test_corpus_synth.py:192-194`
reads "The identity constants a regenerated case must carry. Written out here
rather than imported from the generator ...", and round 6 inserted the lossy
bounds directly beneath it with no blank line, so it now heads
`CHROMA_SUBSAMPLED_MAX`. The constants it was written for, `FIXED_DATE`,
`FIXED_TIME` and `INSTANCE_UID_ARC`, are at lines 207 to 209 with nothing above
them. A blank line and a move.

**N3, `NOT_PIP` says three entries are "each covered by one of the tests below"
and one of them is not.** `test_corpus_synth.py:578-580`. `OpenJPH` has
`test_ci_pins_openjph_to_the_version_it_was_built_with` and `DCMTK` has
`test_ci_does_not_pin_dcmtk`. `OpenJPEG (the library inside it)` has neither,
and `grep OpenJPEG` over the test file returns only the stamp pattern and this
line. It is covered in fact, because the library ships inside the pinned
`pylibjpeg-openjpeg` wheel, but that is a different mechanism and the comment
does not say it. Same enumeration shape as pass 4's D1.

---

## What I could not verify, and why

- **The `corpus-tooling` and `guards` jobs running in GitHub Actions.** No
  runner. I read both files and checked every claim each makes about the other,
  including the single `apt-get install` line, the seven `==` pins and the
  OpenJPH `--branch` tag.
- **That a DCMTK, pyjpegls or OpenJPH bump would in fact move a digest.** One
  version of each on this machine.
- **HLD section 25.1** I read from `docs/hld/22-testing-and-tolerance.md`, not
  from the source `.docx`. Its class two reads "perceptual difference below a
  stated threshold, because chroma subsampling and YBR conversion legitimately
  differ", which is what D1 rests on, and its class one is stated as a maximum
  on a percentile with a hard cap on outliers, which is what the reworked
  docstring claims it is.
- **`docs/SOURCE-POLICY.md`'s "Collections under Attribution-NonCommercial were
  available and were not taken."** A claim about a decision rather than a file.
- **Whether `--fetch` works.** No row carries a url, by design.

---

## What I checked and found correct

**The hard rule.** 1,609 files by magic bytes at offset 128 and by five
suffixes, ignored trees included. Zero DICOM. `gate content` green.

**All eight required commands, exit codes read from the command itself, before
and after the mutation work.** `corpus_check.py` exits 0 in all three modes,
reporting 91 rows, 16 of 16 transfer syntaxes, and 91 verified with 0 missing
and 0 mismatched. `corpus_synth.py --tool-versions` exits 0 with all seven rows
matching. `corpus_tests.py --require-prerequisites` exits 0 with 17 and 38
tests, 55 in all. `gate corpus corpus-tests content provenance prose skills` is
ALL GREEN over 6 gates. `gate --floor` is GREEN, 13 passed and 4 skipped. Note
that `python3 scripts/corpus_synth.py --tool-versions` needs the venv
interpreter, which `corpus/README.md` says and the runner resolves.

**Every file carries the transfer syntax its row claims.** A raw byte walk of
the file meta group from offset 132, parsing explicit-VR element headers with
no pydicom and no DCMTK, reading (0002,0010) out of all 91 files: **0
mismatches**. The same pass recomputed all 91 sha256 digests: **0 mismatches**.
Ninety one rows and 95 files in the tree, the four extra being the `LICENSE`
files, which correctly have no rows.

**Determinism, by my own two-process regeneration.** Two interpreter processes
into two fresh directories, 47 files each, `diff -rq` identical. All 47 digests
equal the committed `synthetic/` and `syntax/` rows, no row lacks a file and no
file lacks a row. The README's sentence "A regenerated corpus therefore has
identical digests and the manifest keeps meaning something" is true, executed
rather than accepted.

**The hand-computed fixture, recomputed from the standard before reading the
file.** Using `shift = HighBit + 1 - BitsStored`, `mask = (1 << BitsStored) -
1` and sign extension from bit `BitsStored - 1`, per PS3.3 C.7.6.3.1.4 and
PS3.5 8.1.1. Right aligned, shift 0: `-2048, 2047, -1, -2047, 0, -16, -16, 15`.
Left aligned, shift 4: `-128, 127, 255, 128, -2048, 2047, -1, -2048`. Both
agree with the file cell for cell, both files carry the eight probe words
byte-identically, and the headers are `(16, 12, 11, 1)` and `(16, 12, 15, 1)`.

**Licences.** All four `LICENSE` files read in full, each naming its own
collection and CC BY 4.0 at `https://creativecommons.org/licenses/by/4.0/`. All
44 real rows carry that pair with an empty `url`, the 47 synthetic rows carry
`MIT OR Apache-2.0` with the Apache URL, and 44 plus 47 is 91. Per-directory
counts 27, 15, 1 and 1. All four DOIs resolve 302 to the collection the
manifest names: `10.7937/SZKB-SW39` to cmb-mml, `10.7937/GHKN-MD15` to
varepop-apollo, `10.7937/c5ke-yx42` to EAY131, `10.7937/DJG7-GZ87` to cmb-crc.

**Every number in `docs/SOURCE-POLICY.md`'s TCIA paragraph, against the live
NBIA API.** CMB-MML 1,156 series all CC BY 4.0, VAREPOP-APOLLO 1,549 all CC BY
4.0, CMB-CRC 2,537 all CC BY 4.0, EAY131 30,293 of which 14,494 CC BY 4.0 and
15,799 null on both fields, those being exactly 14,395 RTSTRUCT and 1,404 SEG.
Each of the four `SeriesInstanceUID` values in `corpus/README.md` returns the
Creative Commons Attribution 4.0 International License with its URI and
`ImageCount` 27, 15, 1 and 1, which is the README's real-layer table from a
second source. Every figure is exact.

**Stated counts.** I swept the changed files for spelled and numeric counts and
checked each against the thing it counts. "The three transform cases" at
`test_corpus_synth.py:202` is right, `jpeg_extended_12`, `j2k_lossy` and
`htj2k_lossy`, the near-lossless case being excluded two lines above and the
chroma case taking the other bound. "the two lossy JPEG cases" at
`corpus_synth.py:144` is right. "The MR series above is Implicit VR Little
Endian and the other three are Explicit VR Little Endian" at
`corpus/README.md:216` is right, read out of the bytes. The rest of pass 5's
sweep I re-spot-checked and found unchanged and correct. I found no count this
diff gets wrong.

**The gate plumbing, driven rather than read.** A failing test injected into
`test_corpus_check.py` makes `bin/ocelli.sh gate corpus-tests` exit 1 and print
FAIL. Dropping the RLE row from the manifest makes `--coverage` exit 1 while
the digest pass still exits 0, and `gate corpus` is red, which is the `&&`
chaining that `bin/ocelli.sh:129` claims and explains.

**Voice rules.** No em-dash or en-dash in any changed file, by direct byte
grep. `gate prose` green over 49 tracked files.

**Working tree and corpus.** `diff -rq` against copies taken before this pass
reports `target/.rustc_info.json` and nothing else in the worktree, and nothing
at all in the corpus. `git status --porcelain` is the eighteen-entry list it
started as, plus this report as a nineteenth intent-to-add entry.

**One gap considered and not raised.** Nothing automated re-proves that the
generator still reproduces the committed manifest. The suite compares two fresh
runs to each other, `corpus_check.py` compares the files on disk to the
manifest, and no gate joins the two. I proved the property by hand above and
`.github/workflows/ci.yml:130-143` reasons explicitly about why CI cannot,
naming it a local operation. Raising it as a finding would be re-litigating a
decision the diff states and defends, so it is here as an observation.

---

## Mutations run, and what went red

Each was applied to the working tree, the suite or check run, then reverted
from a copy taken beforehand with the revert proved by `diff -q`.

| # | Mutation | Result |
|---|----------|--------|
| M1 | `J2K_COMPRESSION_RATIO` 20 to 3000 | **RED**, closeness, `j2k_lossy` |
| M2 | `JPEG_QUALITY` 90 to 1 | **RED**, closeness, two cases |
| M4 | `ci.yml` drop the `pylibjpeg-openjpeg` pin | **RED**, the pin test |
| M4b | `ci.yml` drop the `pyjpegls` pin | **RED**, the pin test |
| M7 | `ci.yml` apt pin `dcmtk=3.6.7-9.1build4` | **RED**, the DCMTK test |
| M8 | `BUILT_WITH` gains an entry CI never installs | **RED**, the pin test |
| M9 | `ci.yml` `dcmtk` written `dcmtk-tools` | **RED**, the DCMTK test |
| M10 | delete `coverage()`'s unknown-syntax arm | **RED**, the new coverage test |
| M11 | drop `mark_lossy` on the HTJ2K lossy case | **RED**, the declaration test |
| M11b | mark `jpegls_lossless` lossy as well | **RED**, the declaration test |
| M12 | the YBR case takes `TRANSFORM_MAX` | **RED**, closeness, `jpeg_baseline_rgb8` |
| M13 | `j2k_lossy` declares `ISO_14495_1` | **GREEN**. Smell S1 |
| M16 | `HTJ2K_QSTEP` 0.001 to 0.01 | **GREEN**. Nitpick N1 |
| M17 | `JPEG_QUALITY` 90 to 10 | **RED**, but only on the chroma case |
| M18 | blank the unknown-syntax problem message | **RED**, the new coverage test |
| M20 | `NOT_PIP` gains `pydicom` | **RED**, the pin test |
| M21 | `.203` out of `LOSSY_TRANSFER_SYNTAXES` | **RED**, two tests |
| M22 | `HTJ2K_QSTEP` 0.001 to 0.1 | **RED**, closeness, `htj2k_lossy` |
| M23 | a failing test injected, gate plumbing | **RED**, `gate corpus-tests` exit 1 |
| M24 | the RLE row dropped, coverage only | **RED**, `gate corpus` exit 1 |

Sixteen went red. Two of the four green ones are findings, M17 is red for the
wrong reason rather than green, and none is a test that cannot fail.

**On the hunt for a test that cannot fail.** I found none, which agrees with
passes 4 and 5 over a different mutation set. M9, M11b, M12, M18, M20, M21,
M22, M23 and M24 were each written by removing or corrupting the thing a guard
protects rather than by reading the guard, and every one went red on the
assertion whose name says it should. What I found instead is one assertion that
is weaker than the property it is about, S1, which is the same class pass 5
found twice and a milder one than the vacuous tests passes 1 to 3 found.

---

## Verdict

**Not clean.** 1 defect, 1 smell, 3 nitpicks.

The trajectory is 4 and 4, 3 and 2, 4 and 2, 1 and 1, 0 and 2, and now 1 and 1.
The defect count going back up by one is not a regression in the work. Round 6
touched one thing this story had not touched before, the justification of a
threshold, and that is where the finding is.

**The corpus itself is sound and this pass established that by measurement, not
by reading.** I regenerated it twice in separate processes and matched all 47
digests, reparsed every file's transfer syntax and digest without the tools
that wrote them, recomputed the sixteen fixture cells from PS3.3 before opening
the file, read the lossy declarations out of all eighteen syntax files, and
checked every TCIA figure against the live API. Nothing in the corpus, the
generator, the manifest, the fixture, the licences or the coverage checker is
wrong.

Round 6's four changes all do what the round claims. The three mutations pass 5
left green are red, the coverage arm has a test that survives the message
attack, the lossy declarations bind in both directions, and the closeness check
is now a worst-pixel bound whose four quoted measurements I reproduced to the
digit. That last one is the important verdict of this pass: **the thresholds
are set from what the encoders produce, not fitted to what passes.**

What is left is one sentence explaining a threshold with the wrong cause, and
one assertion checking the shape of a value rather than the value. Both are
small, both are two-line fixes, and both were found by measuring rather than by
reading, which is the only way either would have been found.
