# F-009, implementation review, pass 4

**Reviewer**: independent agent, wrote neither the work nor pass 1, 2 or 3
**Diff reviewed**: working tree, base cd74768, branch `work/f-009-claude`
**Result**: 1 defect, 1 smell, 2 nitpicks

The hard rule first. A Python walk over every file in the worktree outside
`.git`, ignored trees included, reading bytes 128 to 131 of each of **1,607**
files and separately matching `.dcm`, `.dicom`, `.ima`, `.img` and `.dc3` by
suffix: **zero hits on both**. No DICOM is in this repository. All 91 manifest
rows resolve and verify under `$OCELLI_CORPUS_DIR`.

Every finding below was reproduced by running something. Sixteen mutations were
applied and every one reverted from a copy taken first, each revert proved with
`diff -q`. At the end of this pass `diff -rq` against a copy of the whole
worktree taken before it started reports two differences, both
`scripts/__pycache__/*.pyc`, which the interpreter rewrote. No tracked or
staged file differs, and `diff -rq` over the corpus reports nothing at all.

---

## Earlier findings, and whether each is genuinely resolved

### Pass 3 D1, the unasserted producer-to-case attribution. Resolved, structurally, and I wrote the mutation myself

`EXTERNAL_ENCODERS` at `scripts/corpus_synth.py:168-180` is now keyed by
transfer syntax and holds no filename. `cases_for()` at `:193-197` derives the
names from `SYNTAX_CASES`. Pass 3's M5, a swap of case names inside the table,
is no longer expressible.

The lie in the new currency is a swap of syntaxes between the two producers
that leave no version stamp. I wrote it two ways:

| Mutation | What it says | Result |
|---|---|---|
| MU-A | DCMTK owns `.80` and `.81`, pyjpegls owns `.50 .51 .57 .70` | **RED**, 6 failures |
| MU-B | single swap, `.57` and `.80` exchanged | **RED**, 2 failures, both named |

Both land on `test_the_stampless_producers_are_told_apart_by_the_fingerprint`,
which reads `DerivationDescription` out of every compressed case and asserts
presence on exactly `cases_for("DCMTK, dcmcjpeg")` and absence everywhere else.
Asserted both ways, so it binds in both directions.

I also checked the remaining moves rather than only the two I was asked for.
Moving `.90` to OpenJPH goes red on the version-pattern test, moving `.90` to
DCMTK goes red on both the stamp test and the fingerprint test, and moving
`.80` to OpenJPEG goes red on the version-pattern test. Every pairwise
reassignment in the table is now caught by the bytes.

**One list of filenames, not two: confirmed.** `grep '"[a-z0-9_]*\.dcm"'` over
`scripts/corpus_synth.py` returns filename literals only in `SYNTAX_CASES`,
the three `REFERENCE_*` constants, the two `SYNTAX_REFERENCE` overrides and
`generate_syntax_layer`'s dispatch calls, which must name what they encode.
`EXTERNAL_ENCODERS` holds none. The attribution the dispatch implies and the
attribution the table declares are joined by the tests above rather than left
side by side.

### Pass 3 S2, `--tool-versions` re-typing the attribution. Resolved

`scripts/corpus_synth.py:841-843` now loops `for producer in
EXTERNAL_ENCODERS` and `for name in cases_for(producer)`. The hand-typed
"Which rows: DCMTK owns the four jpeg_* cases" string is gone. The derived
path runs only on drift, so I forced drift with MU-D and watched it print the
rows correctly from the table.

### Pass 3 D2, the README claiming CI runs the `corpus-tests` gate. Resolved

`corpus/README.md:253` is now
`bin/ocelli.sh gate corpus-tests      # what /verify runs, see below for CI`.
`grep -n "ocelli.sh" .github/workflows/ci.yml` returns lines 98, 169 and 213.
Line 169 is the comment "NOT `bin/ocelli.sh gate corpus-tests`, deliberately"
and line 176 is the direct runner call. The `/verify` half is true, the gate is
in `--floor` and `gate --floor` reports 13 passed and 4 skipped over 17 gates.

### Pass 3 D3, the decoder-independence claim. Resolved, and I resolved the decoder graph myself

`corpus/README.md:313-317` now reads "Four cases are encoded and decoded by the
same library", naming the `j2k_*` pair through OpenJPEG and the `jpegls_*` pair
through pyjpegls, and says the `jpeg_*` and `htj2k_*` cases go through a
different library from the one that encoded them. `ojph_expand`, which nothing
in this repository runs, is gone.

`pydicom.pixels.decoders.base.get_decoder` resolution, measured, against the
encoder each case actually had:

| Syntax | Case | Encoder | Decoder plugins, in order |
|---|---|---|---|
| `.50` | `jpeg_baseline_rgb8` | DCMTK | `('pylibjpeg',)` |
| `.51` | `jpeg_extended_12` | DCMTK | `('pylibjpeg',)` |
| `.57` | `jpeg_lossless_p14` | DCMTK | `('pylibjpeg',)` |
| `.70` | `jpeg_lossless_p14_sv1` | DCMTK | `('pylibjpeg',)` |
| `.80` | `jpegls_lossless` | pyjpegls | `('pyjpegls', 'pylibjpeg')` |
| `.81` | `jpegls_near_lossless` | pyjpegls | `('pyjpegls', 'pylibjpeg')` |
| `.90` | `j2k_lossless` | OpenJPEG | `('pylibjpeg',)` |
| `.91` | `j2k_lossy` | OpenJPEG | `('pylibjpeg',)` |
| `.201` | `htj2k_lossless` | OpenJPH | `('pylibjpeg',)` |
| `.202` | `htj2k_lossless_rpcl` | OpenJPH | `('pylibjpeg',)` |
| `.203` | `htj2k_lossy` | OpenJPH | `('pylibjpeg',)` |

The four JPEG cases decode through `pylibjpeg-libjpeg` and were encoded by
DCMTK, so they are independent. The three HTJ2K cases decode through
`pylibjpeg-openjpeg` and were encoded by OpenJPH, so they are independent. The
two JPEG 2000 cases decode through the same OpenJPEG that encoded them, and
pydicom takes the first plugin for JPEG-LS, which is the same CharLS that
encoded them. The gap entry names exactly those four. Correct.

### Pass 3 D4, "CI pins seven of these eight". Resolved by deletion of the number

`corpus/README.md:107` is now "**CI pins everything here except DCMTK**". True:
`ci.yml:124-128` pins seven Python packages with `==` and `ci.yml:156` pins
OpenJPH to the `0.31.0` tag, leaving only the distribution DCMTK. The sentence
cannot go stale when a plugin is added, which is what a count could not manage.

### Pass 3 S1, `CHANGELOG.md`'s gate count

Excluded by instruction. Not counted here.

### Pass 3 N1, three assertions satisfied by the always-printed counts. Resolved, and I redid the measurement

All three are rekeyed off the problem message. I blanked each problem message
in `scripts/corpus_check.py`, leaving `problems.append` in place so the exit
status still goes to 1, which is the condition under which pass 3 measured two
of them green:

| Mutation | Message blanked | Result |
|---|---|---|
| MU-C1 | the "has no colour or ultrasound case" problem | **RED**, 2 tests |
| MU-C2 | the "has no monochrome 16-bit case" problem | **RED**, 1 test |
| MU-C3 | the "declares no transfer syntax" problem | **RED**, 1 test |

Pass 3's M15 and M16 are now red. The new keys are `"the corpus has no colour
or ultrasound"`, `"the corpus has no monochrome 16-bit"` and
`"real/ghost.dcm: declares no transfer syntax"`, none of which is a substring
of any always-printed line.

### Pass 3 N2 and N3. Resolved

`EncoderProvenance.STAMPS` is matched with `re.IGNORECASE`
(`scripts/tests/test_corpus_synth.py:485`). The version test uses
`assertIsNotNone(re.search(...))` with the filename in the message rather than
`assertRegex` over a 16 KB object, and carries a comment saying why.

---

## Deletions, and whether any was load-bearing

Round 4's own note says the deletions were the README's encoder table, its
counts and the three "asserts every cell" sentences. I checked each against
whether anything else records the fact.

| Deleted | Was it load-bearing | Verdict |
|---|---|---|
| The README's per-case encoder table | No. The fact lives in `EXTERNAL_ENCODERS`, `corpus/README.md:124-129` points at it by name, and every cell of it is now asserted against the bytes by MU-A and MU-B | **Correct deletion** |
| "asserts every cell of it", three times | No. The sentences were the claim pass 3 falsified. What replaced them is a test, not a shorter sentence | **Correct deletion** |
| "CI pins seven of these eight" | The substance survives as "everything here except DCMTK", which I verified against `ci.yml` | **Correct deletion** |
| `ojph_expand` from the gap list | No. Nothing in this repository runs it, and the decode path is `pylibjpeg-openjpeg`, which the new sentence covers correctly | **Correct deletion** |
| "the four jpeg_* cases, pyjpegls the two jpegls_*" from `--tool-versions` | No. The same output is now derived from the table at print time and I watched it print under MU-D | **Correct deletion** |

No deletion removed a fact that nothing else records. Nothing that was deleted
needed a test written for it that was not written.

---

## New defects

### D1, the README enumerates four chroma cases and the corpus has five

**Where**: `corpus/README.md:157-159`.

> Every byte of chroma in this corpus is generated by this repository:
> `synthetic/us_ybr_full_422.dcm`, the two `sc_rgb_*` cases and
> `syntax/jpeg_baseline_rgb8.dcm`.

**What**: `syntax/reference_rgb8.dcm` is missing from the list. Read out of the
file, it is `PhotometricInterpretation` RGB with `SamplesPerPixel` 3 and
`BitsAllocated` 8. Its manifest row carries `synthetic, colour,
syntax-reference`. It is a corpus case with a digest, not a stray file, and it
is the base `jpeg_baseline_rgb8.dcm` is encoded from and compared against.
There are five synthetic cases carrying a `colour` token and the sentence names
four.

**Why it is wrong**: microscope class 3. The general claim, that all the chroma
is synthetic, is true. The colon makes the list read as the enumeration of
where that chroma is, and it is not complete, so a reader working out which
cases exercise the colour path misses one. This is the story's declared repeat
offence, an enumeration contradicted by the thing it enumerates, and the
contradiction is visible in the check's own output four paragraphs above:
`--coverage` prints `colour or ultrasound rows: 6 (1 real, ...)`, and six minus
the one real greyscale ultrasound is five synthetic, against four named.

**Evidence**:

```text
$ awk -F'\t' 'NR>1 && $4 ~ /colour/ {print $1"\t"$4}' corpus/manifest.tsv
synthetic/sc_rgb_interleaved.dcm    synthetic, colour, planar-config-0
synthetic/sc_rgb_planar.dcm         synthetic, colour, planar-config-1
synthetic/us_ybr_full_422.dcm       synthetic, us, colour, ybr-full-422
syntax/jpeg_baseline_rgb8.dcm       synthetic, colour, transfer-syntax
syntax/reference_rgb8.dcm           synthetic, colour, syntax-reference

case                        PI             SPP  BA
syntax/reference_rgb8.dcm   RGB            3    8
$ grep -c reference_rgb8 corpus/README.md
0
```

**Fix**: add the case, or drop the list and let the sentence stand on its
general claim, which is true and cannot drift. The second is shorter and is
what round 4's own direction asks for.

---

## New smells

### S1, `--tool-versions` claims the toolchain matches `corpus/README.md` and never reads `corpus/README.md`

**Where**: `scripts/corpus_synth.py:845`, `:819`, `:745-758`, against
`corpus/README.md:86-94`.

```python
    print("\nOK: the toolchain matches what corpus/README.md records")
```

**What**: the comparison at `:828` is against `BUILT_WITH`, a dict in the
generator. Nothing in `scripts/`, `bin/` or `.github/` parses the README's
prerequisites table. `grep -rn BUILT_WITH` returns the definition, the one
loop that reads it, and a CI comment. So the seven version numbers exist twice,
by hand, with nothing joining them, and the command that exists to tell a
toolchain bump from a corrupted corpus asserts an agreement it never checked.

Two consequences, and the second bites even with the tables in step:

1. A README edit that is not mirrored into `BUILT_WITH` is invisible.
2. The README table names **nine** prerequisites. `BUILT_WITH` covers seven
   entries, of which `OpenJPEG (the library inside it)` is not a README row of
   its own. `pylibjpeg` 2.1.0, `pylibjpeg-libjpeg` 2.4.0 and `pylibjpeg-rle`
   2.2.0 are recorded by the README and never asked. On this machine all three
   happen to match, so the sentence is true here, but it would print unchanged
   if they did not.

**Why it is a smell and not a defect**: every value agrees today. I checked all
nine against the installed distributions and all nine match. The mechanism that
keeps them agreeing is what is absent.

**Why it is worth a finding**: pass 3's own "Verified clean" section states
"the `corpus/README.md` version table is asserted by `--tool-versions`". A
careful independent reader has already been misled by this sentence once, which
is the evidence that it is doing work it cannot do.

**Evidence**, mutation MU-D. `corpus/README.md`'s `pydicom` row changed from
3.0.2 to 9.9.9, nothing else touched:

```text
$ python scripts/corpus_synth.py --tool-versions
pydicom                              3.0.2        3.0.2
...
OK: the toolchain matches what corpus/README.md records
EXIT=0

$ python3 scripts/corpus_tests.py --require-prerequisites     -> 0, 50 passed
$ bin/ocelli.sh gate corpus corpus-tests content provenance prose skills -> 0
```

Green everywhere, with the README recording a version no machine has.

**Fix**: parse the table and assert it against `BUILT_WITH` in
`scripts/tests/test_corpus_synth.py`, which is the cheap direction and makes
the printed sentence true. Or narrow the sentence to name `BUILT_WITH` and stop
claiming the README.

---

## Nitpicks

**N1, the gap list's "Four" is counted over eleven cases, not twelve.**
`corpus/README.md:313`. `syntax/rle_lossless.dcm` is encoded by pydicom's own
RLE encoder and decoded by pydicom's own RLE decoder, `get_decoder` for
`1.2.840.10008.1.2.5` resolving to `('pydicom', 'pylibjpeg')` with pydicom
first. By the same measure the bullet applies to `j2k_*` and `jpegls_*`, RLE is
a fifth same-library case. The exclusion is defensible and I would not change
the number: `test_the_internally_encoded_case_is_plugin_independent` asserts
both encode plugins produce byte-identical output, and I separately measured
both decode plugins producing identical arrays, so RLE is better evidenced than
the four that are named. One clause saying the count is over the external
encoder output would stop the next pass recounting it.

**N2, "fails a row that carries neither" understates the guard.**
`corpus/README.md:49-50`. The check fails a row missing **either** structural
token, not only one missing both. Verified by the suite:
`test_a_row_with_no_tolerance_class_token_fails` adds a row whose category is
just `synthetic` and gets exit 1, and `test_a_row_that_is_neither_real_nor_synthetic_fails`
adds one whose category is just `mono16` and gets exit 1. The sentence as
written is true and weaker than the code, and the table row directly above it
says "Exactly one", so the stricter rule is at least reachable.

---

## What I could not verify, and why

- **The `corpus-tooling` job running in GitHub Actions.** No runner. I read
  both files and every claim each makes about the other is true, including the
  `--require-prerequisites` call at `ci.yml:176` and the seven `==` pins plus
  the OpenJPH tag.
- **That a DCMTK or pyjpegls bump would in fact move a digest.** One version of
  each on this machine. The direction of the argument is sound and the
  attribution is now asserted against the bytes, which is what can be shown
  here.
- **HLD section 25.1** I read from `docs/hld/22-testing-and-tolerance.md`. Its
  two classes and the phrase "chroma subsampling and YBR conversion
  legitimately differ" are exactly what the README and `corpus_check.py` quote,
  and it does state no tolerance for 8-bit monochrome, so the README's sentence
  about the ultrasound case being absorbed into class two by modality is
  correct. I did not check it against the source `.docx`.
- **`docs/SOURCE-POLICY.md`'s "Collections under Attribution-NonCommercial were
  available and were not taken."** A claim about a decision, not about a file.
  Not enumerable here.
- **The EAY131 series-level licence figures** in `docs/SOURCE-POLICY.md`. Pass
  3 measured all of them against the NBIA API and found them exact. I
  re-checked the four DOIs and the four `LICENSE` files rather than repeating
  the collection-wide census.

---

## What I checked and found correct

**The hard rule.** 1,607 files by magic bytes at offset 128 and by five
suffixes, ignored trees included. Zero DICOM. `gate content` green.

**All eight required commands, exit codes read from the command itself.**
`corpus_check.py` in all three modes exits 0, reporting 91 rows, 16 of 16
transfer syntaxes and 91 verified with 0 missing and 0 mismatched.
`corpus_synth.py --tool-versions` exits 0 with all seven rows matching.
`corpus_tests.py --require-prerequisites` exits 0 with 16 and 34 tests, 50 in
all, one more than pass 3 saw.
`gate corpus corpus-tests content provenance prose skills` is ALL GREEN over 6
gates. `gate --floor` is GREEN, 13 passed and 4 skipped.

**Every file carries the transfer syntax its row claims.** A raw byte parse of
the file-meta group from offset 132, walking explicit-VR element headers, with
no pydicom and no DCMTK, reading (0002,0010): **91 rows, 0 mismatches**. The
same pass recomputed all 91 digests independently: 0 mismatches.

**Determinism, by my own two-process regeneration.** Two interpreter processes
into two fresh directories, 47 files each, `diff -rq` identical, and all 47
digests equal to the committed `synthetic/` and `syntax/` rows. Nothing
generated lacks a row and no row lacks a generated file.

**The hand-computed fixture, recomputed from the standard.** I derived all
sixteen cells from PS3.3 C.7.6.3.1.4 and PS3.5 8.1.1, using
`shift = HighBit + 1 - BitsStored`, `mask = (1 << BitsStored) - 1` and sign
extension from bit `BitsStored - 1`. `RIGHT_ALIGNED` (shift 0):
`-2048, 2047, -1, -2047, 0, -16, -16, 15`. `LEFT_ALIGNED` (shift 4):
`-128, 127, 255, 128, -2048, 2047, -1, -2048`. Both agree with the file cell
for cell, every worked derivation in the comment block is correct line by line,
and the `0x800F` note about the discarded low nibble is right. The two cases
carry byte-identical pixel data and headers `(16, 12, 11, 1)` and
`(16, 12, 15, 1)`.

**Coverage fails four ways, against the real manifest.** Header-only manifest,
the RLE row dropped, the RLE row's syntax changed to `1.2.840.10008.1.2.4.100`,
and the RLE row's `transfer_syntax` blanked. All four exit 1. The dropped and
unknown cases name both the missing registry UID and the unknown one, and the
blank case names the row by path. Manifest restored and proved with `diff -q`
after every one.

**Licences.** All four `LICENSE` files read in full: each names its own
collection and CC BY 4.0 at `https://creativecommons.org/licenses/by/4.0/`. All
44 real rows carry that pair with an empty `url`, and no `LICENSE` file has a
manifest row. Row counts per directory 27, 15, 1, 1, matching the README's
real-layer table. All four DOIs resolve 302 to the right collection:
`10.7937/SZKB-SW39` to cmb-mml, `10.7937/GHKN-MD15` to varepop-apollo,
`10.7937/c5ke-yx42` to EAY131, `10.7937/DJG7-GZ87` to cmb-crc. The 47 synthetic
rows carry `MIT OR Apache-2.0` with the Apache URL, and 44 plus 47 is the 91
rows in the file.

**The encapsulation gap claim, read out of the files.** All twelve compressed
cases: Basic Offset Table length 4 with one offset `[0]`, exactly one fragment,
`NumberOfFrames` 1. "Every compressed case is one frame in one fragment with a
populated Basic Offset Table" is exact.

**The `docs/SOURCE-POLICY.md` pydicom quote.** "I believe there is no
restriction on using any of these files in this manner" is verbatim at line 11
of the installed pydicom's `data/test_files/README.txt`.

**The four conditions for done.** Row per case with a real sha256 and an
actionable licence pair, `corpus_check.py` green, both HLD 25.1 classes covered
in the real layer as well as overall, and 16 of 16 registry syntaxes.

**Stated counts across every file this diff touches.** Swept for spelled counts
and checked each against the thing it counts. "Eleven of the twelve compressed
rows" and "six of those eleven" at `corpus_synth.py:745-747` are right, 12
compressed, 11 external, 4 plus 2 stampless. "fourteen instances" at `:649` is
right, 16 syntax cases less the mono12 and rgb8 ones. "Sixteen of them" at
`corpus_check.py:48` matches the 16-entry registry list. The registry test's
"the two native ones, deflate, the retired big-endian one, RLE, four JPEG, two
JPEG-LS, two JPEG 2000 and three HTJ2K" sums to 16 and matches the typed list.
"Three of those are load-bearing and all three default to empty" at
`README.md:332` is right, `--modality`, `--transfer-syntax` and `--category`
all default to `""` and `load()` rejects an empty modality. The only
enumeration that fails is D1.

**Voice rules.** No em-dash or en-dash anywhere in the changed files, by direct
grep. The only semicolons in `corpus/README.md` are inside a fenced bash block.
`gate prose` green over 47 tracked files.

**Working tree and corpus.** `diff -rq` against copies taken before this pass
reports two `.pyc` caches and nothing else in the worktree, and nothing at all
in the corpus. `git status --porcelain` is the same fifteen-entry list it
started as.

---

## Mutations run, and what went red

Each was applied to the working tree, the affected suite or check run, then
reverted from a copy taken beforehand with the revert proved by `diff -q`.

| # | Mutation | Result |
|---|----------|--------|
| MU-A | swap all syntaxes between DCMTK and pyjpegls | **RED**, fingerprint test, 6 failures |
| MU-B | swap `.57` and `.80` only | **RED**, fingerprint test, 2 named failures |
| MU-C1 | blank the "no colour or ultrasound case" problem message | **RED**, 2 tests |
| MU-C2 | blank the "no monochrome 16-bit case" problem message | **RED**, 1 test |
| MU-C3 | blank the "declares no transfer syntax" problem message | **RED**, 1 test |
| MU-D | `corpus/README.md` pydicom 3.0.2 to 9.9.9 | **GREEN**, everywhere. This is smell S1 |
| MU-E | `HighBit` 11 to 10 on the right-aligned fixture | **RED**, 1 failure and 1 error |
| MU-F | `PixelRepresentation` 1 to 0 on both signed fixtures | **RED**, 4 failures |
| MU-G | `PixelSpacing` to `["0.5", "0.5"]` | **RED** |
| MU-H | `PixelSpacing` transposed to `["0.25", "0.5"]` | **RED** |
| MU-I | stop stripping the producing tool's `ImplementationVersionName` | **RED**, 4 DCMTK subtests |
| MU-J | stop re-deriving series and frame of reference | **RED**, both attributes |
| MU-K | leave encapsulated HTJ2K `PixelData` as OW | **RED**, 3 subtests |
| MU-L | header-only manifest | `--coverage` **RED**, exit 1 |
| MU-M | drop the only RLE row | `--coverage` **RED**, names the UID |
| MU-N | RLE row's syntax to `1.2.840.10008.1.2.4.100` | `--coverage` **RED**, names both |
| MU-O | RLE row's `transfer_syntax` blanked | `--coverage` **RED**, names the path |
| MU-P | print the chroma NOTE unconditionally | **RED**, 1 test |
| MU-Q | neuter the contradictory-token check | **RED**, 1 test |

Only MU-D stayed green, and it is a finding. **I found no test that cannot
fail.** MU-I, MU-J, MU-K, MU-P and MU-Q were written specifically to hunt for
one, each by removing the thing a guard protects rather than by reading the
guard, and all five went red. The five vacuous tests this story produced across
three passes are the ones round 4 and its predecessors fixed, and no new one
replaced them.

---

## Verdict

**Not clean.** 1 defect, 1 smell, 2 nitpicks.

The count has fallen from 4 and 4, to 3 and 2, to 4 and 2, to 1 and 1, and the
character of the findings has changed with it. Round 4's instruction to delete
rather than explain worked: every deletion I checked was safe, no deleted fact
went unrecorded, and the one finding with teeth was fixed structurally rather
than described. `EXTERNAL_ENCODERS` is now a table whose every cell an attacker
would have to change the bytes to satisfy, which is the strongest form the fix
could have taken.

Neither finding this pass is wrong behaviour, and neither is in the code round 4
wrote. D1 is an enumeration in `corpus/README.md` that has been one short since
the pass 1 chroma remediation and has survived three reviews, including two that
read the same paragraph. S1 is a coupling that never existed and that pass 3
recorded as existing. Both are the same shape as everything this story has
produced: a sentence that is more confident than the thing behind it. The
corpus, the generator, the determinism, the fixture, the licences and the
coverage guard are sound, and this pass reached that conclusion by regenerating,
reparsing and mutating rather than by reading.
