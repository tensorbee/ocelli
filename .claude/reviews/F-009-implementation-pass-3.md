# F-009, implementation review, pass 3

**Reviewer**: independent agent, wrote neither the work nor pass 1 nor pass 2
**Diff reviewed**: working tree, base cd74768
**Result**: 4 defects, 2 smells, 3 nitpicks

The hard rule first. A Python walk over every file in the worktree outside
`.git`, ignored trees included (`.claude/scratch/`, `scripts/__pycache__/`,
`scripts/tests/__pycache__/`, `target/`), reading bytes 128 to 131 of each of
**1,604** files and separately matching `.dcm`, `.dicom`, `.ima`, `.img` and
`.dc3` by suffix: **zero hits on both**. No DICOM is in this repository. The
corpus is entirely under `$OCELLI_CORPUS_DIR` and all 91 manifest rows verify
there.

Every finding below was reproduced by running something. Every mutation was
reverted and the revert proved with `diff -q`. At the end of this pass
`diff -rq` against a copy taken before it started reports one difference,
`target/.rustc_info.json`, which `bin/ocelli.sh gate --floor` rewrote. No
tracked or staged file differs.

---

## Earlier findings, and whether each is genuinely resolved

### Pass 1

**D1, `--coverage` is not run by CI. Resolved.** `.github/workflows/ci.yml:108`
is `python3 scripts/corpus_check.py --coverage` in the `guards` job. I
re-confirmed the subsumption rather than reading it: a two-field row makes both
`--coverage` and `--manifest-only` exit 1 with the same shape message, because
`main()` calls `load()` before dispatching.

**D2, nothing executes the Python tests. Resolved.** `scripts/corpus_tests.py`,
the `corpus-tests` gate arm and the `corpus-tooling` CI job. Reproduced in full
under S1 below.

**D3, the OpenJPEG size comment. Resolved.** The comment at
`scripts/corpus_synth.py:113-121` now states 32 as the floor and 64 as that
doubled for headroom, with the derivation (six resolution levels is five
decompositions, so `2**5`). Every clause of it is true.

**D4, the NBIA licence claim. Resolved, and I re-measured every figure.** See
the licence section below. All four collections and all four series were
queried again and every number in `docs/SOURCE-POLICY.md` is exact to the unit.

**S1, the chroma gap. Resolved, and now enforced.** The manifest row carries
`greyscale-8bit, chroma-untested`, `corpus/README.md` has the real-layer table
with **no chroma** on the US row, and `--coverage` prints the advisory NOTE.
Round 3 added the `CONTRADICTORY_TOKENS` check on top, which pass 2 raised as
N2. Verified: adding a `colour` token to that row now exits 1 naming the path
and both tokens.

**S2, an empty `transfer_syntax`. Resolved.** Reproduced, exit 1, named per
path.

**S3, encoder versions. Substantively resolved.** The `EXTERNAL_ENCODERS` table
is correct against the bytes in every particular I could check, and the
`corpus/README.md` version table is asserted by `--tool-versions`. What is not
resolved is the claim made *about* the test, which is new defect D1.

**S4, the degenerate syntax series. Resolved.** Read back from 18 files in the
real corpus's `syntax/` directory: 18 distinct `SeriesInstanceUID`, 18 distinct
`FrameOfReferenceUID`, 1 `StudyInstanceUID`.

### Pass 2

**D1, the encoder-version prose. Resolved as to the facts, and I checked all
four cells against the bytes myself.** A byte scan of every file in `syntax/`
for `OpenJPH`, `OpenJPEG`, `DCMTK`, `OFFIS`, `CharLS`, `pyjpegls` and any
`Ver`- or `version`-plus-digits run:

```
htj2k_lossless.dcm       OpenJPH=b'OpenJPH Ver 0.31.0.'
htj2k_lossless_rpcl.dcm  OpenJPH=b'OpenJPH Ver 0.31.0.'
htj2k_lossy.dcm          OpenJPH=b'OpenJPH Ver 0.31.0.'
j2k_lossless.dcm         OpenJPEG=b'OpenJPEG version 2.5.2'
j2k_lossy.dcm            OpenJPEG=b'OpenJPEG version 2.5.2'
jpeg_baseline_rgb8.dcm   IJG=b'IJG'          (DerivationDescription, not a version)
jpeg_extended_12.dcm     IJG=b'IJG'          (DerivationDescription, not a version)
jpeg_lossless_p14.dcm    -
jpeg_lossless_p14_sv1.dcm -
jpegls_lossless.dcm      -
jpegls_near_lossless.dcm -
rle_lossless.dcm         -
```

Four DCMTK cases and two pyjpegls cases carry no version, three OpenJPH and two
OpenJPEG do. `DerivationDescription` is present on exactly the four DCMTK cases
and on nothing else, recording selection value, point transform and
compression ratio, plus the IJG quality factor on the two lossy ones. The RLE
case is plugin independent: `RLELosslessEncoder.available_plugins` is
`('pydicom', 'pylibjpeg')` and both produce bytes identical to each other and
to `syntax/rle_lossless.dcm`. Every count in the prose (`eleven of the twelve`,
`six of the eleven`, `two of the four`) is correct.

**D2, the stale test count. Resolved.** `scripts/corpus_tests.py` now states no
count and says why. A repository-wide sweep for a stated count of tests, gates,
cases, rows or syntaxes in the files this diff touches found nothing else that
the code can contradict, with two exceptions that are findings below (README
defect D4 and CHANGELOG smell S1).

**D3, the DCMTK pin comment. Resolved, and the reasoning holds.** Checked all
three legs. `ubuntu-latest` is Ubuntu 24.04 (`actions/runner-images` README,
`ubuntu-latest` or `ubuntu-24.04` on the 24.04 row). `packages.ubuntu.com/noble/dcmtk`
is `3.6.7-9.1build4`, against the 3.7.0 that `BUILT_WITH` and the README record.
The `corpus-tooling` job genuinely compares no digest against the manifest: it
generates into a `tempfile.TemporaryDirectory` and the only manifest-touching
job is `guards`, which runs `--coverage`, a manifest-shape and coverage check
that reads no bytes. So a 3.6.7 there cannot fail anything the job asserts, and
adding `--tool-versions` to CI would fail on one row every run and tell nobody
anything actionable. The comment now says all of that and every sentence of it
is true.

**S1, the gate count in `.claude/commands/verify.md`. Resolved there.** The
sentence now names the command rather than a number. The adapter is in sync:
`shasum -a 256 .claude/commands/verify.md` is
`3db61dcaa62cb9157a0c13aa260f82ed97a480340dea450d0569b4ff1a763b8a`, which is
what `.agents/skills/verify/SKILL.md:10` records, and
`python3 scripts/sync_agent_skills.py --check` reports `OK: 20 adapters match
their canonical sources`. The same sentence survives elsewhere, which is smell
S1 below.

**S2, the CI job could go green on a skip. Resolved.** Three-way result
reproduced exactly, with `PATH=/usr/bin:/bin` so that neither `dcmcjpeg` nor
`ojph_compress` resolves:

| Invocation | Exit | What it printed |
|---|---|---|
| `bin/ocelli.sh gate corpus-tests` (the old CI call) | **0** | `SKIPPED corpus-tests`, `GREEN 0 passed, 1 skipped` |
| `python3 scripts/corpus_tests.py --require-prerequisites` (the new CI call) | **1** | `FAIL: a prerequisite is absent and is named above` |
| `python3 scripts/corpus_tests.py` (what `gate --floor` runs) | **3** | `SKIPPED, not on PATH: dcmcjpeg (...), ojph_compress (...)` |

The skip is named in all three, never reported as a pass, and `--floor` still
reports `13 passed, 4 skipped`. `-DOJPH_ENABLE_TIFF_SUPPORT=OFF` is at
`ci.yml:160` and the `command -v ojph_compress` assertion at `ci.yml:164`.

**N1, `--tool-versions` on an absent tool. Resolved.** With DCMTK and OpenJPH
off `PATH` it prints the full table with `absent` in the `here` column, marks
both `<- differs`, and exits 1. No traceback.

**N2, the inert `chroma-untested` token. Resolved.** Now a named contradiction,
verified red.

**N3, the `$OCELLI_PYTHON` fallback prose. Resolved.** The README now scopes
the sentence with "**for the generator suite**" and says explicitly that the
coverage suite runs under `sys.executable` regardless.

**N4, the note's condition. Resolved.** The README now carries the parenthesis
"(With no real class-two row at all, coverage fails outright for a louder
reason.)"

---

## New defects

### D1, the producer-to-case half of `EXTERNAL_ENCODERS` is not asserted, and three places say every cell is

**Where**: `scripts/corpus_synth.py:32-33` (module docstring),
`scripts/corpus_synth.py:186-191` (the comment above the table),
`corpus/README.md:137-141`.

**What**. The three sentences are:

> `EXTERNAL_ENCODERS` below is the table, and it is asserted against the
> written bytes by `EncoderProvenance`, **so this paragraph cannot drift from
> the corpus**

> `EncoderProvenance` in `scripts/tests/test_corpus_synth.py` asserts **every
> cell of it** against the bytes actually written, so it cannot drift from the
> corpus again.

> `EncoderProvenance` in `scripts/tests/test_corpus_synth.py` **asserts every
> cell of it** against the bytes of the written files

What `EncoderProvenance` actually asserts is two things: that the union of the
claimed case names plus `INTERNAL_ENCODER_CASE` equals the set of compressed
cases, and that a producer carrying a version pattern leaves a match while a
producer carrying `None` leaves none of seven literal stamps. Neither touches
**which** producer owns a case among the two that leave nothing. DCMTK and
pyjpegls both carry `None`, so their four and two case names can be exchanged
freely and the table stays green.

**Why it is wrong**: microscope class 3, a false claim in prose, and class 4, a
guard that looks authoritative over more than it reaches. It is also the exact
half the surrounding prose calls the dangerous one: "**The cases with no
version stamp are the ones to watch, not the ones with one.**" Those six are
precisely the cases whose attribution nothing checks. The remediation for pass
2's D1 replaced a wrong hand-written paragraph with a table plus a claim that
the table is checked, and the claim is broader than the check.

**Evidence**, mutation M5. Swap the four DCMTK names with the two pyjpegls
names, so the table says DCMTK produced the JPEG-LS cases and pyjpegls produced
two of the JPEG ones:

```
###### MU5: swap one DCMTK case with one pyjpegls case (attribution lie)
Ran 4 tests in 0.142s
OK
--- and the whole suite ---
OCELLI-SUITE test_corpus_check.py ran=16 failures=0 errors=0 skipped=0
OCELLI-SUITE test_corpus_synth.py ran=33 failures=0 errors=0 skipped=0
OK: corpus tooling tests
```

Green, all 49 tests. `EXTERNAL_ENCODERS` is also never read by `generate()`.
`generate_syntax_layer` dispatches through `encode_with_dcmcjpeg`,
`encode_with_ojph` and `encode_with_pydicom` directly, and
`encode_with_pydicom` covers the pyjpegls, OpenJPEG and pydicom-RLE cases
alike, so the table is a second list beside the dispatch with nothing joining
them.

**Fix**: the discriminator already exists in the bytes.
`DerivationDescription` is present on exactly the four DCMTK cases and absent
from every other file in `syntax/`, which I verified independently. Asserting
that, or deriving the table from the generator's own dispatch, would make the
claim true. Alternatively narrow the three sentences to what is checked.

### D2, `corpus/README.md` says CI runs a gate that CI deliberately does not run

**Where**: `corpus/README.md:277`.

```bash
bin/ocelli.sh gate corpus-tests      # what CI runs, and what /verify runs
```

**What**: CI does not run that gate. `grep -n "ocelli.sh gate"
.github/workflows/ci.yml` returns lines 98 (`gate docs`) and 213 (`gate wasm`)
and nothing else. The `corpus-tooling` job runs
`python3 scripts/corpus_tests.py --require-prerequisites` at `ci.yml:176`,
under a comment at `ci.yml:169` that begins "NOT `bin/ocelli.sh gate
corpus-tests`, deliberately." The same README says so itself 25 lines later:
"So that job calls the runner directly with this flag."

**Why it is wrong**: microscope class 3, and it is the remediation's own
by-product. Round 3 changed the CI invocation precisely so that the gate is not
what CI runs, and left the line that says it is. The `/verify` half of the
clause is true, `corpus-tests` is in `--floor`.

**Fix**: `# what /verify runs. CI calls the runner directly, see below.`

### D3, the gap list credits two encoders with independently decoding their own output

**Where**: `corpus/README.md:337-340`, under "What this corpus still does not
have".

> **One decoder for JPEG 2000.** Every other compressed case is decoded by a
> tool other than the one that encoded it, RLE and the JPEG family through
> DCMTK and the HTJ2K trio through `ojph_expand`.

**What**: the four `jpeg_*` cases are encoded by DCMTK `dcmcjpeg`
(`scripts/corpus_synth.py:705`), so decoding them "through DCMTK" is the same
tool, not a different one. The three `htj2k_*` cases are encoded by
`ojph_compress` (`scripts/corpus_synth.py:734`), and `ojph_compress` and
`ojph_expand` are two binaries from one install:

```
/opt/homebrew/Cellar/openjph/0.31.0/bin/ojph_compress
/opt/homebrew/Cellar/openjph/0.31.0/bin/ojph_expand
```

So the sentence's own two examples both fail its own test, by the same measure
it applies to JPEG 2000 one sentence later ("encoded and decoded by the same
OpenJPEG"). `ojph_expand` is also not used by anything in this repository, only
named in this sentence.

**Why it is wrong**: microscope class 3, in the one section whose entire
purpose is an honest statement of what the corpus cannot evidence. It
understates the count of single-library links rather than overstating it, which
is the direction that gets believed.

**What is actually true**, and why the fix is cheap: the automated conformance
decode does have independence for both, through different tools from the ones
named. `pydicom.pixels.decoders.base.get_decoder` resolves `.57` and `.70` to
`('pylibjpeg',)`, which is `pylibjpeg-libjpeg`, not DCMTK, and `.201` to
`('pylibjpeg',)`, which is `pylibjpeg-openjpeg` wrapping OpenJPEG 2.5.2, not
OpenJPH. Naming those two makes the sentence true. Worth noting while the
sentence is being rewritten: `.80` and `.81` resolve to
`('pyjpegls', 'pylibjpeg')` and pydicom takes the first, so JPEG-LS is
CharLS-encoded and CharLS-decoded in the automated path, a second single-
library link the gap list does not name.

### D4, "CI pins seven of these eight" is neither seven nor eight

**Where**: `corpus/README.md:108`.

> **CI pins seven of these eight and does not pin DCMTK**, which comes from the
> Ubuntu distribution and is 3.6.7 there.

**What**: the prerequisites table immediately above names **nine** things:
`pydicom`, `numpy`, `pylibjpeg`, `pylibjpeg-libjpeg`, `pylibjpeg-rle`,
`pylibjpeg-openjpeg`, `pyjpegls`, DCMTK and OpenJPH. CI pins **eight** of them,
seven with `==` at `ci.yml:124-128` and OpenJPH to the tag `0.31.0` at
`ci.yml:156`. Only DCMTK is unpinned. Counted by table rows instead of by named
prerequisite it is six of seven. Neither reading gives seven of eight, which is
reachable only by dropping OpenJPH from the count, and OpenJPH is both in the
table and pinned by CI.

**Why it is wrong**: microscope class 3, and it is this story's declared repeat
offence, a stated count contradicted by the thing it counts. The number
originates in pass 2's own D3 write-up and was copied into the README rather
than recounted. The substance is right and only the arithmetic is wrong, which
is what makes it survive a read.

**Evidence**:

```
$ sed -n '124,128p;156p' .github/workflows/ci.yml
            "pydicom==3.0.2" "numpy==2.5.2" \
            "pylibjpeg==2.1.0" "pylibjpeg-libjpeg==2.4.0" \
            "pylibjpeg-openjpeg==2.5.0" "pylibjpeg-rle==2.2.0" \
            "pyjpegls==1.5.1"
          git clone --depth 1 --branch 0.31.0 \
```

**Fix**: "CI pins everything here except DCMTK", which is short, true, and
cannot go stale when a plugin is added.

---

## New smells

### S1, "Seventeen gates" survives in `CHANGELOG.md`, and F-009 is what made it wrong under both readings

**Where**: `CHANGELOG.md:23`, inside the `## Unreleased` section, so it is not
one of the frozen released sections that `scripts/prose_check.py` exempts.

> - Seventeen gates behind `bin/ocelli.sh gate`, and a CI floor that runs every
>   one that needs no GPU and no corpus.

At cd74768 the `GATES` array had 18 entries, 17 of them non-GPU, so the
sentence was wrong against the list and right against the non-GPU subset. This
diff adds `corpus-tests`, `bin/ocelli.sh gate --list` now prints 19 gate rows
and 18 of them are non-GPU, and the sentence is wrong against both.

This is a smell rather than a defect on the same grounds pass 2 used for the
identical sentence in `.claude/commands/verify.md`: it is pre-existing and not
in the diff. It is here because the pass 2 remediation removed the number from
one of the two places it lives and left the other, because F-009 is the change
that moved the number, and because nothing checks it. Prose in `CHANGELOG.md`
is outside `prose_check.py`'s scope entirely, so this will not be caught later
either.

**Fix**: delete the word, as was done in `verify.md`.

### S2, `--tool-versions` re-types the encoder attribution instead of deriving it from the table

**Where**: `scripts/corpus_synth.py:854-856`, against the table at `:192-204`.

```python
              f"apart before anyone edits the manifest. Which rows: DCMTK "
              f"owns the four jpeg_* cases, pyjpegls the two jpegls_* cases, "
              f"OpenJPEG the two j2k_* cases, OpenJPH the three htj2k_* ones.")
```

Every fact in that string is in `EXTERNAL_ENCODERS`, 650 lines above, in a
module that says of that table "**This table is the source the prose is derived
from**". This is a hand-typed fourth copy of the claim, in the same file, in
the one command a developer runs when their digests have already moved. It is
correct today, I checked all four clauses against the filenames and the bytes.
It is the same shape as the thing that was wrong three ways in round 2, and
nothing joins the two.

It compounds with D1: with the attribution unasserted, a swap in the table
would leave this string right and the table wrong, or the reverse, with no test
able to tell.

**Fix**: build the sentence from `EXTERNAL_ENCODERS` at print time, or drop the
"Which rows" clause and point at the table.

---

## Nitpicks

**N1.** Three coverage tests assert a message string that the always-printed
counts lines already contain, so the naming half of each is satisfied by a
passing run. `test_a_manifest_with_no_colour_or_ultrasound_fails` asserts
`assertIn("colour or ultrasound", text)` while line 244 always prints
`colour or ultrasound rows: N`. `test_a_manifest_with_no_monochrome_16_bit_fails`
asserts `assertIn("monochrome 16-bit", text)` while line 239 always prints
`monochrome 16-bit rows: N`. `test_a_row_with_no_transfer_syntax_is_named`
asserts `assertIn("transfer syntax", text)` while line 236 always prints
`transfer syntaxes: N of 16`, of which that is a substring. Proved, not read:
replacing each problem message with `XXXX` leaves all three green. This is
exactly pass 1's N1, which round 2 fixed for
`test_real_rows_covering_only_one_class_fail` by keying on a longer sentence
and left in these three. Not a defect, because `assertEqual(status, 1)` still
binds in each and the third also asserts the path, so no test is vacuous.

**N2.** `EncoderProvenance.STAMPS` lists `CharLS` and `charls` but only
uppercase `DCMTK` and `OFFIS`, and `re.search` is case sensitive, so a future
lowercase `dcmtk` stamp would slip through the "leaves none" assertion. One
`re.IGNORECASE` closes it.

**N3.** `test_a_producer_that_claims_a_version_leaves_one` uses
`assertRegex(self.raw(name), pattern)` on the whole file, so a failure prints
the entire ~16 KB DICOM object per subtest. Mutation M1 produced four of those.
Asserting on `re.search(...) is not None` with the filename in the message
keeps the failure readable.

---

## What I could not verify, and why

- **The `corpus-tooling` job running in GitHub Actions.** No runner. What I did
  check: `ubuntu-latest` maps to Ubuntu 24.04 in the current
  `actions/runner-images` README, noble's `dcmtk` is `3.6.7-9.1build4`, and the
  job's own claims about what it does and does not compare are true by reading
  both files. The `OJPH_ENABLE_TIFF_SUPPORT` question pass 2 left open is now
  closed by `-DOJPH_ENABLE_TIFF_SUPPORT=OFF`, and `command -v ojph_compress`
  makes a build that installs outside `PATH` fail the step rather than the
  suite.
- **That a DCMTK or pyjpegls bump would in fact move a digest.** One version of
  each on this machine. The direction of the argument is sound and the
  manifest is reproducible on this toolchain, which is what can be shown here.
- **HLD section 25.1's wording** I read from
  `docs/hld/22-testing-and-tolerance.md`, which says exactly what the code and
  the README quote. I did not check it against the source `.docx`, which is
  outside this repository.
- **`docs/SOURCE-POLICY.md`'s "Collections under Attribution-NonCommercial were
  available and were not taken."** True in the general sense that TCIA hosts
  CC BY-NC collections. It is a claim about a decision, not about a file, and I
  did not enumerate what was considered.
- **`gate prose` over the pass 2 report.** That file is untracked, so
  `git ls-files` does not reach it and `gate prose` never scored it. Scanned
  directly at the start of this pass it carried 7 semicolons, the known item
  excluded from these counts. The integrator's mechanical fix landed while this
  pass was running and a direct re-scan now returns 0 problems for all three
  review files. Separately, and not this story's doing:
  `prose_check.py --staged` sees nothing at all for an intent-to-add file,
  because `git diff --cached --name-only` lists no `-N` entry here, so the
  pre-commit path would miss a newly added review while `gate prose`, which
  uses `git ls-files`, catches it.

---

## What I checked and found correct

**The hard rule.** 1,604 files by magic bytes at offset 128 and by five
suffixes, ignored trees included. Zero DICOM. `gate content` green.

**The hand-computed fixture, recomputed from the standard.** I derived all
sixteen cells myself from PS3.3 C.7.6.3.1.4 and PS3.5 8.1.1, using
`shift = HighBit + 1 - BitsStored`, `mask = (1 << BitsStored) - 1` and sign
extension from bit `BitsStored - 1`, before reading the table.
`RIGHT_ALIGNED` (shift 0, sign bit 0x800): `0xF800` to -2048, `0x07FF` to 2047,
`0x0FFF` to -1, `0x0801` to -2047, `0x8000` to 0, `0x7FF0` to -16, `0xFFF0` to
-16, `0x800F` to 15. `LEFT_ALIGNED` (shift 4): -128, 127, 255, 128, -2048,
2047, -1, -2048. Both agree with the file on every cell, and every worked
derivation in the comment block is correct line by line. Read back out of the
generated files, the two cases carry byte-identical pixel data and headers
`(16, 12, 11, 1)` and `(16, 12, 15, 1)`, and unpack to exactly those two rows.
The expected values are not read back from the generator and the generator
contains no unpacking code they could have come from.

**Every file carries the transfer syntax its row claims.** A raw byte parse of
the file-meta group from offset 132, walking explicit-VR element headers with
no pydicom and no DCMTK, reading (0002,0010): **91 rows, 0 mismatches**.

**Determinism, by my own two-process regeneration.** Two separate interpreter
processes into two fresh directories: 47 files each, `diff -rq` identical, and
all 47 digests equal to the committed manifest rows. No generated file lacks a
manifest row and no `synthetic/` or `syntax/` manifest row was not generated.

**Licences, the ones that cannot be fixed later.** All four `LICENSE` files
read in full: each names its own collection and CC BY 4.0 at
`https://creativecommons.org/licenses/by/4.0/`. All 44 real rows carry
`CC BY 4.0` and that URL, with an empty `url` as the README says, and no
`LICENSE` file has a manifest row. Row counts per directory 27, 15, 1, 1,
matching both the README table and the files on disk. All four DOIs resolve 302
to the right collection page: `10.7937/SZKB-SW39` to cmb-mml,
`10.7937/GHKN-MD15` to varepop-apollo, `10.7937/c5ke-yx42` to EAY131,
`10.7937/DJG7-GZ87` to cmb-crc. `getSeriesMetaData` for each of the four
`SeriesInstanceUID` values returns CC BY 4.0 with collections and modalities
CT/MR/DX/US matching the README table cell for cell. The aggregate figures in
`docs/SOURCE-POLICY.md` are exact: CMB-MML 1,156 series all CC BY 4.0,
VAREPOP-APOLLO 1,549 all, CMB-CRC 2,537 all, EAY131 30,293 total with 14,494
CC BY 4.0 and 15,799 null on both fields, those being RTSTRUCT 14,395 and SEG
1,404.

**The real-layer table, read out of the files.** Each of the four directories
holds exactly one `SeriesInstanceUID` and one pixel-module combination.
`ct_cmb_mml` 27 files, CT, Explicit VR LE, 16-in-16 signed MONOCHROME2, GE
MEDICAL SYSTEMS. `mr_eay131` 15 files, MR, **Implicit VR LE**, 12-in-16
unsigned MONOCHROME2, SIEMENS. `dx_varepop` 1 file, DX, Explicit VR LE,
12-in-16 MONOCHROME2. `us_cmb_crc` 1 file, US, Explicit VR LE, 8-bit
MONOCHROME2, `SamplesPerPixel` 1, GE Healthcare, no chroma. Every cell matches
the README.

**The encoder table, cell by cell against the bytes.** See pass 2 D1 above. All
four producer rows, all eleven external cases, the version patterns and the
`DerivationDescription` distribution are as stated. The RLE case is
byte-identical under both pydicom plugins and equal to the corpus file.

**The coverage guard, made to fail five ways.** Header-only manifest, the RLE
row dropped, the RLE row's syntax changed to a non-registry UID, the RLE row's
syntax blanked, and a `colour` token added beside `chroma-untested`. All five
exit 1 and all five name the row or the UID. Manifest restored and proved with
`diff -q` after every one.

**The gate arm and its chaining.** `corpus` is `--coverage && corpus_check.py`,
so a coverage break with intact digests fails the gate rather than being
reported green by the last command's status. `corpus-tests` needs no corpus, so
it is correctly inside `--floor`.

**Gates.** `gate corpus corpus-tests content provenance skills prose` ALL GREEN,
6 gates. `gate --floor` GREEN, 13 passed and 4 skipped (docs, lint, types,
wasm, all for pre-existing stated reasons). `corpus_check.py` green in all
three modes over the real corpus, 91 verified 0 missing 0 mismatched.
`corpus_synth.py --tool-versions` green, all seven rows matching.
`sync_agent_skills.py --check` green over 20 adapters. The full 49-test suite
green, `ran=16` and `ran=33`.

**Prose rules.** No em-dash or en-dash in any file this diff touches, by direct
grep. `prose_check.py` green over 45 tracked files, its scope covering
`corpus/README.md` and `docs/SOURCE-POLICY.md` by exact name and
`.claude/reviews/` by prefix.

**Working tree.** `diff -rq` against the copy taken before this pass reports
only `target/.rustc_info.json`, rewritten by cargo during `gate --floor`.
`git status --porcelain` is the same fifteen-entry list it started as.

---

## Mutations run, and what went red

Each was applied to the working tree, the affected suite or check run, then
reverted from a copy taken beforehand and the revert proved with `diff -q`.

| # | Mutation | Result |
|---|----------|--------|
| M1 | give DCMTK a version pattern in `EXTERNAL_ENCODERS` | **RED**, `test_a_producer_that_claims_a_version_leaves_one`, 4 subtests |
| M2 | move `jpegls_lossless.dcm` to the OpenJPEG row | **RED**, same test, named subtest |
| M3 | drop `j2k_lossy.dcm` from the table | **RED**, `test_every_compressed_case_has_exactly_one_declared_producer` |
| M4 | point `INTERNAL_ENCODER_CASE` at a JPEG-LS case | **RED**, 3 failures across two tests |
| M5 | swap the four DCMTK names with the two pyjpegls names | **GREEN**, whole suite. This is defect D1 |
| M6 | `HighBit` 11 to 10 on `ct_signed_12in16_right` | **RED**, 1 failure and 1 error |
| M7 | `PixelRepresentation` 1 to 0 on both signed cases | **RED**, 4 failures |
| M8 | `NON_SQUARE_SPACING` to `["0.5", "0.5"]` | **RED**, `test_pixel_spacing_is_non_square_where_it_should_be` |
| M9 | header-only manifest | `--coverage` **RED** exit 1, all sixteen syntaxes named |
| M10 | drop the only RLE row | `--coverage` **RED** exit 1, names `1.2.840.10008.1.2.5` |
| M11 | RLE row's syntax to `1.2.840.10008.1.2.4.100` | `--coverage` **RED**, names the missing UID and the unknown one |
| M12 | RLE row's `transfer_syntax` blanked | `--coverage` **RED**, named per path |
| M13 | `colour` added beside `chroma-untested` | `--coverage` **RED**, names the path and both tokens |
| M14 | `PATH` stripped of `dcmcjpeg` and `ojph_compress` | gate exit 0, `--require-prerequisites` exit 1, bare runner exit 3 |
| M15 | replace the "colour or ultrasound" problem message with `XXXX1` | **GREEN**, which is nitpick N1 |
| M16 | replace the "monochrome 16-bit" problem message with `XXXX2` | **GREEN**, which is nitpick N1 |

M5, M15 and M16 are the three that stayed green, and each is a finding. Every
other mutation went the direction it should. The four defects came from reading
files against the bytes, against the distribution and against each other, not
from a mutation, except D1 which M5 settled.

---

## Verdict

**Not clean.** 4 defects, 2 smells, 3 nitpicks.

The trend is still downward in severity and the findings are still
concentrated in the newest surface, which this round is the encoder-provenance
table, the CI job and the README paragraphs written to explain them. Nothing
this pass found is wrong behaviour. All four defects are false sentences, three
of them in `corpus/README.md`, and two of the four (D2 and D4) are sentences
that round 3's own remediations made false or copied without recounting. That
is the pattern the microscope warns about under class 3: a remediation that
corrects one sentence and adds three explaining it ships three new claims.

The corpus itself, the generator, the determinism, the fixture, the licences
and the coverage guard are all sound and were checked by doing rather than by
reading. D1 is the only finding with any teeth beyond prose, and its fix is one
assertion against a discriminator that is already in the bytes.
