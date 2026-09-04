# F-009, implementation review, pass 7

**Reviewer**: independent agent, wrote neither the work nor pass 1, 2, 3, 4, 5
or 6
**Diff reviewed**: working tree, base cd74768, branch `work/f-009-claude`
**Result**: 0 defects, 0 smells, 2 nitpicks

The hard rule first. A Python walk over every file in the worktree outside
`.git`, ignored trees included, reading bytes 128 to 131 of each of **1,604**
files and separately matching `.dcm`, `.dicom`, `.ima`, `.img`, `.dc3` and
`.dicm` by suffix, case insensitively: **zero hits on both**, run at the start
of this pass and again at the end. `git ls-files` matches no DICOM suffix. No
DICOM is in this repository. All 91 manifest rows resolve and verify under
`$OCELLI_CORPUS_DIR`.

Thirty two mutations were applied. At the end of this pass `diff -rq` against a
copy of the whole worktree reports **no difference at all** outside `.git`,
`__pycache__` and `target`, and `diff -rq` over the corpus reports nothing.
`git status --porcelain` is the nineteen-entry list it started as, plus this
report. Everything below was run with `PYTHONDONTWRITEBYTECODE=1` and a cleared
cache, per the note pass 5 left.

**A method note, because it cost me an hour and it is a trap the next pass
should not repeat.** The scratchpad path this harness hands out is **not**
per-pass. It still held pass 5 and pass 6's working files, including a
directory named `wt-baseline`, so my `cp -a <worktree> <scratchpad>/wt-baseline`
nested a fresh copy **inside** a stale one instead of creating it, and my first
mutation script used `set -e` in a way that exited before its revert. Two
generator mutations were left applied for about ten minutes. I detected it
because the unmutated suite failed, reconstructed the pristine file three ways
and proved the reconstruction by `diff`, and the tree is now byte-identical to
its pre-pass state as shown above. The lesson worth keeping: **a mutation
harness must verify the tree is clean before it mutates, not only after**, and
the one used here does.

---

## Round 7's changes, verified

### D1's fix, the chroma bound. Every number reproduces, and the name is better

I re-encoded `reference_rgb8.dcm` myself with `dcmcjpeg` 3.7.0 at quality 90,
changing one flag at a time, decoded each through the same path the test uses,
and built the 8-bit `MONOCHROME2` control from the green channel of the same
ramp. **All five claimed figures reproduce to the digit.**

```text
+cr   RGB, no YBR at all       max 1 of 255  0.003922   decoded PI=RGB
+s4   4:4:4 YBR                max 3 of 255  0.011765   decoded PI=YBR_FULL
      shipped, default 4:2:2   max 4 of 255  0.015686   decoded PI=YBR_FULL_422
+np   "4:1:1"                  max 4 of 255  0.015686   decoded PI=YBR_FULL_422
      8-bit MONOCHROME2        max 1 of 255  0.003922
```

So does the transposed figure. Reading "a transposed variant of the same shape"
as the ramp's two gradients exchanged with the 64 by 96 frame kept, which is
what the words say, `dcmcjpeg --encode-baseline --quality 90` gives 5 of 255,
`0.019608`. Transposing the image instead gives `0.011765`. And `0.03 / 0.019608
= 1.53`, so "1.5 times the worst measured" is right.

**Is the rewritten comment true?** Yes, on the sentence that matters. "The
colour case needs its own bound because of its container, not its subsampling"
is the sharpest true statement available here, and I checked it against the
alternative rather than accepting it. The colour case costs **4 levels**. The
HTJ2K case costs **41 levels**, sixteen times more, and sits comfortably under
`TRANSFORM_MAX`. The whole difference is the denominator: 4 of 255 is 0.0157,
and the same 4 levels in the 12-bit container would be 0.00098. The container
is the cause of the magnitude. Subsampling is 1 of the 4 levels, which is what
pass 6 found and what round 7 now says.

**The "three quarters" clause, which I nearly reported and then did not.**
Read as YBR conversion's *marginal* cost with subsampling left on, it is 2 of 4
levels, a half, and the clause would be wrong. Read as "remove this cause
entirely and see what is left", which is what `+cr` measures and what the
comment quotes, turning YBR off takes the case from 4 levels to 1, so YBR and
everything it enables is 3 of 4 and subsampling is the remaining 1 of 4. Those
two sum to one and the second reading is exactly the ratio
`0.011765 / 0.015686 = 0.750000`. The clause is true under the counterfactual
its own measurement supports, and the comment prints all four raw numbers so a
reader can compute either. Reporting it would have been inventing a finding.

**Is `YBR_EIGHT_BIT_MAX` better than `CHROMA_SUBSAMPLED_MAX`, or merely
different?** Better, on two independent grounds. It matches the selector, which
keys on `ds.PhotometricInterpretation.startswith("YBR")` and not on any
subsampling condition, and it names the container, which the measurement shows
is the reason the fraction is an order larger. The old name pointed at the
smallest of the three contributors. No stale reference to the old name survives
anywhere outside the earlier pass reports.

**The bound is load-bearing in both directions.** Y1 (`0.03` to `0.01`) and Y2
(the selector broken so the colour case takes `TRANSFORM_MAX`) both go red on
`jpeg_baseline_rgb8.dcm`, and Y3 (`TRANSFORM_MAX` to `0.0001`) goes red on two
transform cases.

### The three nitpicks, taken

| Pass 6 | Claim | Verified |
|---|---|---|
| N1 | `TRANSFORM_MAX` says where its bound comes from | `TRANSFORM_MAX`'s comment now ends "the worst of them rounded up an order" |
| N2 | the orphaned comment restored | line 210's comment now heads `FIXED_DATE`, with a blank line above it, and the bounds comment heads `YBR_EIGHT_BIT_MAX` |
| N3 | `NOT_PIP`'s claim corrected | now "OpenJPH and DCMTK have a test below each. OpenJPEG has none, because it ships inside the pinned pylibjpeg-openjpeg wheel" |

N3 is now true as a matter of fact and not only as a matter of wording.
`grep` finds `test_ci_pins_openjph_to_the_version_it_was_built_with` and
`test_ci_does_not_pin_dcmtk` and no OpenJPEG test, `ci.yml` installs pip pins,
apt `dcmtk` and an OpenJPH source build and nothing else, and the OpenJPEG
version the generator records comes out of the `Created by OpenJPEG version
2.5.2` stamp that the `pylibjpeg-openjpeg` 2.5.0 wheel's bundled library writes
into `j2k_lossless.dcm` and `j2k_lossy.dcm`.

I checked for the duplicated-comment failure the microscope warns about. No
comment line over 25 characters appears twice in the changed test file except
the section rule, and no duplicated multi-line block exists.

---

## The codestream classifier, judged

```python
def codestream_method(self, frame: bytes) -> str:
    if frame[:2] == b"\xff\xd8":                     # ISO/IEC 10918-1 SOI
        return ("ISO_14495_1" if b"\xff\xf7" in frame[:64]  # SOF55
                else "ISO_10918_1")
    if frame[:4] == b"\xff\x4f\xff\x51":              # SOC then SIZ
        rsiz = struct.unpack(">H", frame[6:8])[0]
        return "ISO_15444_15" if rsiz & 0x4000 else "ISO_15444_1"
    return f"unrecognised codestream {frame[:4].hex()}"
```

**The marker classification is correct.** `FFD8` is SOI, ISO/IEC 10918-1
B.1.1.3. `FFF7` is SOF55, the JPEG-LS start of frame, which is what makes a
JPEG-LS stream distinguishable from a DCT one inside the same SOI framing.
`FF4F` then `FF51` is SOC then SIZ, ISO/IEC 15444-1 A.4.1 and A.5.1, and the
SIZ segment is `FF51`, `Lsiz` at offset 4, **`Rsiz` at offset 6**, so
`frame[6:8]` is the right two bytes. Bit `0x4000` in `Rsiz` is the
extended-capabilities flag that says a CAP marker is present, which is what
ISO/IEC 15444-15 requires and what a plain Part 1 codestream does not set. All
four strings it can return are Defined Terms of PS3.3 C.7.6.1.1.5.1, and each
is the right term for its algorithm.

**It cannot misclassify any of the sixteen cases.** I ran the classifier myself
over **all sixteen**, including the eleven it is never called on, reading the
markers out of the files with my own parser:

```text
jpeg_baseline_rgb8     ffd8ffe0...  -> ISO_10918_1     declared ISO_10918_1
jpeg_extended_12       ffd8ffe0...  -> ISO_10918_1     declared ISO_10918_1
jpeg_lossless_p14      ffd8ffe0...  -> ISO_10918_1     not lossy, not asserted
jpeg_lossless_p14_sv1  ffd8ffe0...  -> ISO_10918_1     not lossy, not asserted
jpegls_lossless        ffd8fff7...  -> ISO_14495_1     not lossy, not asserted
jpegls_near_lossless   ffd8fff7...  -> ISO_14495_1     declared ISO_14495_1
j2k_lossless           ff4fff51 Rsiz=0x0000 -> ISO_15444_1   not lossy
j2k_lossy              ff4fff51 Rsiz=0x0000 -> ISO_15444_1   declared ISO_15444_1
htj2k_lossless         ff4fff51 Rsiz=0x4000 -> ISO_15444_15  not lossy
htj2k_lossless_rpcl    ff4fff51 Rsiz=0x4000 -> ISO_15444_15  not lossy
htj2k_lossy            ff4fff51 Rsiz=0x4000 -> ISO_15444_15  declared ISO_15444_15
the five native and RLE cases      -> "unrecognised codestream ..."
```

`FFF7` appears at offset 2 in both JPEG-LS files and **nowhere at all** in the
four DCT and lossless-JPEG files, so the `frame[:64]` window is not merely
adequate, the sequence is absent entirely. The `Rsiz` values are `0x0000` and
`0x4000` exactly, so the bit test has no near miss.

**All three term swaps go red, and so does a fourth.** The two JPEG cases carry
a term DCMTK writes rather than one the generator writes, so I mutated the
classifier for those instead.

| # | Mutation | Result |
|---|---|---|
| M2 | `.91` declares `ISO_14495_1`, pass 6's M13 verbatim | **RED**, `j2k_lossy.dcm` |
| M3 | `.203` declares `ISO_15444_1` | **RED**, `htj2k_lossy.dcm` |
| M4 | `.81` declares `ISO_10918_1` | **RED**, `jpegls_near_lossless.dcm` |
| C1 | the classifier's two JPEG arms exchanged | **RED**, three cases |
| C2 | `rsiz & 0x4000` to `rsiz & 0x2000` | **RED**, `htj2k_lossy.dcm` |
| C3 | `Rsiz` read from `frame[4:6]`, which is `Lsiz` | **RED**, `htj2k_lossy.dcm` |
| C5 | SOI written `FFD9` | **RED**, three cases |

Pass 6's S1 is closed. M2 is the mutation that was green last pass.

**Now the harder question: is the classifier a second untested thing?** No, and
the reasons are worth writing down because the answer was not obvious.

- **Every live branch is bound by a case.** The five lossy cases exercise all
  four return arms, `ISO_10918_1` twice and the other three once each, and C1,
  C2, C3 and C5 show each is load-bearing rather than decorative.
- **The fall-through cannot produce a false pass.** It returns a string that is
  not a Defined Term and never can be, so an unrecognised codestream fails the
  `assertEqual` loudly with its magic bytes in the message. I proved the arm is
  reachable in principle by running the classifier on the RLE and native cases,
  which return `unrecognised codestream 02000000` and so on. This is the
  opposite of the silent-default failure this story has produced before.
- **The marker knowledge is triangulated, not asserted once.**
  `test_jpeg_ls_codestreams_carry_sof55`, `test_jpeg_2000_codestreams_start_with_soc_siz`
  and `test_htj2k_codestreams_carry_the_cap_marker` pin the same markers to the
  same transfer syntaxes independently, those tests select by UID,
  `test_declared_syntax_matches_the_file` pins the UID to the file, and
  `test_every_registry_syntax_has_a_case` pins the UID set to
  `corpus_check.REGISTRY_TRANSFER_SYNTAXES`. So marker to syntax is closed
  elsewhere and the classifier adds exactly one new mapping, syntax family to
  Defined Term.

**The author's reasoning for refusing the table is sound and I checked it
rather than accepting it.** A `{syntax: term}` dict would have compared the
generator's term against a copy of the generator's term, keyed on the same
transfer syntax the generator wrote. The classifier compares it against the
bytes. That is a genuinely independent second source, which is the standard
this repository sets for a fixture, and it is why M2 goes red now and did not
before.

The one thing nothing pins is the fall-through's exact return value, which is
nitpick N2 below.

---

## New defects

None.

---

## New smells

None.

---

## Nitpicks

**N1, two loose descriptions in the reworked comment, one of which names the
wrong subsampling scheme.** `scripts/tests/test_corpus_synth.py:196-197` and
`:199`.

The comment says "Through dcmcjpeg at quality 90 with only the sampling flag
changed", but the first of its four measurements changes the **colour space**
flag and not a sampling flag. `dcmcjpeg --help` groups `+cr` under "compression
color space conversion" and `+s4` and `+np` under sampling. One flag at a time
is what was done, and it was not always the sampling one.

The comment then calls `+np` "4:1:1". I parsed the SOF component sampling
factors out of each codestream:

```text
default 4:2:2   PI=YBR_FULL_422  SOF00 comps=[(1,2,1),(2,1,1),(3,1,1)]
+s4  4:4:4      PI=YBR_FULL      SOF00 comps=[(1,1,1),(2,1,1),(3,1,1)]
+np  "4:1:1"    PI=YBR_FULL_422  SOF00 comps=[(1,2,2),(2,1,1),(3,1,1)]
+cr  RGB        PI=RGB           SOF00 comps=[(82,1,1),(71,1,1),(66,1,1)]
```

The luma component under `+np` is `H=2, V=2`, which is **4:2:0**. True 4:1:1
would be `H=4, V=1`. DCMTK's own option is named `--nonstd-411` and the comment
repeats the tool's label, but the codestream is 4:2:0. The load-bearing half of
the sentence, "twice the subsampling still costs 4", is true either way, since
`+np` does halve the chroma again and the measured cost does not move. Two
words.

**N2, the classifier's fall-through is the one thing in it that nothing pins.**
`scripts/tests/test_corpus_synth.py:483`.

```text
C4  return f"unrecognised codestream {frame[:4].hex()}"  ->  return "ISO_10918_1"
    55 tests ... OK                                       <- GREEN
```

Every other arm is red under mutation. This one is unreachable for the five
lossy cases and so nothing would notice if it were softened into a Defined
Term, at which point a future lossy case whose codestream the classifier cannot
parse would be declared JPEG lossy and pass.

**This is a nitpick and not a smell, deliberately.** The shipped arm cannot
produce a false pass, because no Defined Term contains a space or a hex dump,
and the natural evolution of this file is adding a lossy case rather than
editing this line, in which case the arm does exactly the right thing. Writing
it as `self.fail(f"unrecognised codestream {frame[:4].hex()}")` would close it
in the other direction too, and cannot be softened into a passing value at all.
One word.

---

## What I checked and found correct

**The hard rule.** 1,604 files by magic bytes at offset 128 and by six
suffixes, ignored trees included, at the start and end of the pass. Zero DICOM.
`gate content` green.

**All eight required commands, exit codes read from the command itself**, at
the start of the pass and again after every mutation was reverted.
`corpus_check.py` exits 0 in all three modes, reporting 91 rows, 16 of 16
transfer syntaxes, and 91 verified with 0 missing and 0 mismatched.
`corpus_synth.py --tool-versions` exits 0 with all seven rows matching.
`corpus_tests.py --require-prerequisites` exits 0 with 17 and 38 tests, 55 in
all. `gate corpus corpus-tests content provenance prose skills` is ALL GREEN
over 6 gates. `gate --floor` is GREEN, 13 passed and 4 skipped.

**Every file carries the transfer syntax its row claims, checked without
pydicom and without DCMTK.** A raw byte walk from offset 132 parsing explicit-VR
element headers and reading (0002,0010) out of all **91** files: **0
mismatches**. The same pass recomputed all 91 sha256 digests: **0 mismatches**.
Ninety one rows against 95 files in the tree, the four extra being the `LICENSE`
files, which correctly have no rows.

**Determinism, by my own two-process regeneration against the manifest
digests.** Two interpreter processes into two fresh directories, 47 files each,
`diff -rq` byte identical. All 47 digests equal the committed `synthetic/` and
`syntax/` rows, no owned row lacks a file and no generated file lacks a row.

**The hand-computed fixture, recomputed from the standard before opening the
file.** Using `shift = HighBit + 1 - BitsStored`, `mask = (1 << BitsStored) - 1`
and sign extension from bit `BitsStored - 1`, per PS3.3 C.7.6.3.1.4 and PS3.5
8.1.1. Right aligned, shift 0: `-2048, 2047, -1, -2047, 0, -16, -16, 15`. Left
aligned, shift 4: `-128, 127, 255, 128, -2048, 2047, -1, -2048`. Both agree
with the files cell for cell, the headers are `(16, 12, 11, 1)` and
`(16, 12, 15, 1)`, the two files carry the eight probe words byte-identically,
and the test file's `PROBE_WORDS`, `RIGHT_ALIGNED` and `LEFT_ALIGNED` all equal
what I computed.

**Licences.** All four `LICENSE` files read in full, each naming its own
collection and CC BY 4.0 at `https://creativecommons.org/licenses/by/4.0/`. All
44 real rows carry that pair, the 47 synthetic rows carry `MIT OR Apache-2.0`
with the Apache URL, and 44 plus 47 is 91. All four DOIs resolve 302 to the
collection the manifest names: `10.7937/SZKB-SW39` to cmb-mml,
`10.7937/GHKN-MD15` to varepop-apollo, `10.7937/c5ke-yx42` to EAY131,
`10.7937/DJG7-GZ87` to cmb-crc.

**Every lossy measurement in the changed comments, re-measured.** From the
shipped files against `SYNTAX_REFERENCE`, dividing by `(1 << BitsStored) - 1`:
`jpeg_baseline_rgb8` 4 of 255 = `0.015686`, `jpeg_extended_12` 2 of 4095 =
`0.000488`, `j2k_lossy` 2 of 65535 = `0.000031`, `htj2k_lossy` 41 of 65535 =
`0.000626`. The comment's `0.0157`, `0.00049`, `0.00003` and `0.00063` are all
exact. `jpegls_near_lossless` has max error 3, which is `JPEG_LS_NEAR` exactly,
the bound ISO/IEC 14495-1 guarantees.

**HLD 25.1, read from `docs/hld/22-testing-and-tolerance.md`.** Class two is
"perceptual difference below a stated threshold, because chroma subsampling and
YBR conversion legitimately differ", which is both halves and which the
comment, `corpus_check.py`'s coverage note and `corpus/README.md` all quote
consistently.

**Voice rules.** No em-dash or en-dash in any changed file, by direct byte
grep over all eleven. `gate prose` green over 50 tracked files.

**Working tree and corpus.** `diff -rq` against copies taken during this pass
reports **no difference at all** in the worktree outside `.git`, `__pycache__`
and `target`, and nothing at all in the corpus. `git status --porcelain` is the
nineteen-entry list it started as.

**One thing established and deliberately not raised.**
`ds["PixelData"].is_undefined_length = True` in `encode_with_ojph` is a no-op on
the written bytes. I regenerated the whole corpus with it set to `False` and the
three HTJ2K files came out byte-identical, because pydicom's writer forces
undefined length for encapsulated pixel data anyway. The sibling line
`ds["PixelData"].VR = "OB"` **is** load-bearing, and changing it to `OW`
produces three different files and turns
`test_encapsulated_pixel_data_is_ob_with_undefined_length` red. Belt and braces
against a writer that might stop forcing it is defensible engineering rather
than a defect, the property is asserted on the file either way, and the comment
above the pair correctly states PS3.5 for both halves.

---

## What I could not verify, and why

- **The `corpus-tooling` and `guards` jobs running in GitHub Actions.** No
  runner. I read `ci.yml` and checked every claim the test file makes about it,
  including the single `apt-get install` line, the pip pins and the OpenJPH
  source build.
- **True 4:1:1 encoding.** Neither `+np` nor `+n1` produces `H=4, V=1` through
  this DCMTK, so the "twice the subsampling" figure is measured at 4:2:0. On a
  ramp that is smooth horizontally, which is what `rgb_ramp`'s docstring says it
  is for, more horizontal subsampling is the cheap direction, so the conclusion
  does not turn on it.
- **HLD section 25.1** I read from `docs/hld/22-testing-and-tolerance.md`, not
  from the source `.docx`.

---

## Mutations run, and what went red

Each was applied to a tree first proved clean against a verified pristine copy,
the suite run, then the file restored from that copy and the restore proved with
`diff -q`.

| # | Mutation | Result |
|---|---|---|
| P0 | `JPEG_LS_NEAR` 3 to 2 | GREEN, see note |
| M1 | `jls_error=JPEG_LS_NEAR` to `+ 5`, encoder decoupled from bound | **RED** |
| M2 | `.91` declares `ISO_14495_1` | **RED** |
| M3 | `.203` declares `ISO_15444_1` | **RED** |
| M4 | `.81` declares `ISO_10918_1` | **RED** |
| M5 | `NONUNIFORM_OFFSET` 1.25 to 0.0 | **RED** |
| M6 | `UID_ARC` `2.25.` to `1.2.826.` | **RED**, every case |
| M7 | `STUDY_DATE` moved one day | **RED**, every case |
| M8 | `UID_SALT` changed | GREEN, see note |
| M9 | `NON_SQUARE_SPACING` made square | **RED** |
| M10 | one word of `PROBE_WORDS` | **RED**, four fixture tests |
| M11 | `DCMTK_FINGERPRINT` to `ImageComments` | **RED**, four cases |
| M12 | `SERIES_SPACING` 2.5 to 2.0 | **RED**, two tests |
| M13 | the MONOCHROME1 case made MONOCHROME2 | **RED** |
| M14 | `YBR_FULL_422` to `YBR_FULL` | **RED** |
| M15 | `is_undefined_length` False | GREEN, the line is a no-op |
| M15b | `PixelData` VR `OB` to `OW` | **RED**, three cases |
| M16 | `.201` written RPCL instead of LRCP | **RED** |
| M17 | `PlanarConfiguration` fixed at 0 | **RED** |
| M18 | per-frame rescale all taken from index 0 | **RED** |
| M19 | `INTERNAL_ENCODER_SYNTAX` to `.50` | **RED**, three |
| M20 | `.202` out of `REGISTRY_TRANSFER_SYNTAXES` | **RED**, two |
| C1 | classifier's two JPEG arms exchanged | **RED**, three |
| C2 | `Rsiz` bit `0x4000` to `0x2000` | **RED** |
| C3 | `Rsiz` read from `frame[4:6]` | **RED** |
| C4 | fall-through returns `ISO_10918_1` | GREEN. Nitpick N2 |
| C5 | SOI `FFD8` to `FFD9` | **RED**, three |
| Y1 | `YBR_EIGHT_BIT_MAX` 0.03 to 0.01 | **RED** |
| Y2 | the `YBR` selector broken | **RED** |
| Y3 | `TRANSFORM_MAX` 0.005 to 0.0001 | **RED**, two |
| Y4 | `full_scale` fixed at 16 bits | GREEN, a loosening |
| Y5 | `JPEG_QUALITY` 90 to 20 | **RED** |

Twenty seven went red. The five green ones are each explained rather than
counted as coverage:

- **P0** moves `JPEG_LS_NEAR`, which is both the encoder's NEAR setting and the
  bound the test asserts. Green is the correct answer, because ISO/IEC 14495-1
  guarantees max abs error `<= NEAR` for whatever NEAR was used and the test
  says exactly that. **M1 breaks the coupling instead**, leaving the bound at 3
  while the encoder uses 8, and goes red. That is the mutation that proves the
  test.
- **M8** changes the UID salt, which moves every derived UID and therefore every
  digest, but the suite regenerates into a temp directory and compares two fresh
  runs to each other. This is the gap `ci.yml:130-143` names and defends, and
  the brief excludes it.
- **M15** mutates a line that has no effect on the written bytes, proved by
  regenerating and diffing. Covered above.
- **Y4** loosens the denominator rather than tightening it, so green is
  arithmetically inevitable. Y1 and Y3 tighten and both go red.
- **C4** is nitpick N2.

**On the hunt for a test that cannot fail. I found none.** That agrees with
passes 4, 5 and 6 over three different mutation sets, and this one was built by
corrupting the thing a guard protects rather than by reading the guard. Every
mutation that changed observable behaviour went red on the assertion whose name
says it should, and every mutation that did not go red turned out to have
changed nothing observable, or to have loosened a bound rather than tightened
one, or to be the fall-through in N2.

---

## Verdict

**Clean. 0 defects, 0 smells, 2 nitpicks.**

The trajectory is 4 and 4, 3 and 2, 4 and 2, 1 and 1, 0 and 2, 1 and 1, and now
0 and 0. Round 7 touched three things and all three hold. The chroma bound's
comment now names a cause that measurement supports, and I reproduced all six
of its figures rather than reading them. The constant's new name matches its
selector and its cause, which the old one did not. And the shape assertion has
been replaced by something genuinely stronger than the table pass 6 asked for:
the term is now checked against the codestream's own markers, every arm of that
classifier is load-bearing under mutation, its fall-through cannot produce a
false pass, and the mutation that was green last pass is red.

The two nitpicks are four words between them and neither blocks. One is a
subsampling label copied from a tool's option name, the other is a fall-through
that would be better as `self.fail`.

**The corpus, the generator, the manifest, the fixture, the licences and the
coverage checker are sound, and this pass established that by regenerating,
reparsing and mutating rather than by reading.** Two processes into two fresh
directories match each other byte for byte and match all 47 committed digests.
All 91 transfer syntaxes and all 91 digests reparse correctly without the tools
that wrote them. The sixteen fixture cells recompute from PS3.3 before the file
is opened. All four DOIs resolve to the collections the manifest names. And no
DICOM is in this repository, by magic bytes and by suffix, over every file
outside `.git`.

F-009 is done.
