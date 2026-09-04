# F-009, implementation review, pass 1

**Reviewer**: independent agent, did not do the work
**Diff reviewed**: working tree, base cd74768
**Result**: 4 defects, 4 smells, 5 nitpicks

The hard rule first. `git status --ignored`, a suffix sweep and a DICM
magic-byte sweep over every file in the worktree outside `.git` and `target`
all return nothing. No DICOM is present in the repository, staged or otherwise.
The corpus is entirely under `$OCELLI_CORPUS_DIR` and all 91 manifest rows
verify there.

---

## Defects

### D1, `--coverage` is not run by CI, and three places say it is

**Where**: `scripts/corpus_check.py:136-146` (the `coverage` docstring),
`corpus/README.md` (the "Coverage, and how it is checked" section),
`.github/workflows/ci.yml:102`, `bin/ocelli.sh` `--floor` arm.

**What**: the docstring states, of the coverage mode:

> Deviation D-04 means CI has neither a GPU nor the corpus, so coverage is the
> part of F-009 that CI can still see. A manifest that has stopped covering the
> codec registry then fails on the pull request rather than at the moment
> someone tries to answer gate A1.

CI does not run `--coverage`. The only corpus step in `.github/workflows/ci.yml`
is line 102, `python3 scripts/corpus_check.py --manifest-only`, which checks
column count, path safety, digest shape and non-empty licence fields and does no
coverage checking at all. `bin/ocelli.sh gate --floor`, which the file itself
labels "The CI floor", excludes the `corpus` gate by name:

```
case "$name" in oracle|corpus) continue ;; esac
```

That line is unchanged by this diff. So a manifest that drops the only RLE row
does not fail on the pull request. It fails only when a human runs
`/verify --profile feature` locally, which is a different guarantee from the one
the prose asserts.

**Why it is wrong**: microscope class 3 (a false claim in prose) and class 4
(a guard that exists and that CI never reaches). The whole design argument for
building `--coverage` as a manifest-only mode is that CI can run it under D-04,
and the one-line change that would make that true was not made.

**Evidence**:

```
$ grep -n corpus_check .github/workflows/ci.yml
102:      - run: python3 scripts/corpus_check.py --manifest-only

$ bin/ocelli.sh gate --floor 2>&1 | tail -2
GREEN  12 passed, 4 skipped. A skipped gate is NOT a pass.
# corpus is not among the 16 names run
```

Chaining itself is correct and I confirmed it is not masked. With the RLE row
removed, `bin/ocelli.sh gate corpus` exits 1 and reports `FAILED corpus`.

**Fix**: either change `ci.yml:102` to `--coverage` (it calls `load()` first, so
it subsumes `--manifest-only`) or delete the two sentences that claim CI sees it.

---

### D2, nothing executes the 39 new Python tests

**Where**: `scripts/tests/test_corpus_synth.py`, `scripts/tests/test_corpus_check.py`.

**What**: `scripts/tests/` did not exist at cd74768 (`git ls-tree -r
cd74768 -- scripts/tests` is empty). This story creates it and wires it into
nothing. Exhaustive search across `.github/workflows/`, `bin/ocelli.sh`,
`.githooks/`, `.claude/commands/` and every tracked `.yml`, `.sh`, `.py`, `.md`,
`.json` and `.toml`: the only occurrence of `unittest discover` outside the test
files themselves is the instruction in `corpus/README.md:199` for a human to type.
`bin/ocelli.sh gate test` is `cargo test --workspace`, Rust only.

So the hand-computed PS3.3 C.7.6.3.1.4 fixture, the subprocess determinism proof
and all thirteen conformance assertions run only when someone remembers the
command. They will nonetheless be counted as coverage forever.

**Why it is wrong**: microscope class 4, "the defect class that survives a green
suite, because a suite cannot report on what it does not run". It compounds with
D1: after both, no automated gate at any level touches F-009's verification
surface except the pre-existing `--manifest-only` shape check.

**Evidence**:

```
$ grep -rn "unittest\|scripts/tests" .github/workflows/ bin/ocelli.sh .githooks/ .claude/commands/
(no output)
$ git ls-tree -r --name-only cd74768 -- scripts/tests
(no output)
$ grep -n "test)" bin/ocelli.sh
85:    test)        cargo test --workspace ;;
```

Note also that with the wrong interpreter the whole suite exits 0. Plain
`python3` has no pydicom, `test_corpus_synth` reports as one skip, and the
process still returns 0. Under this project's own rule that a skip is not a
pass, any gate arm added must read the skip count and not the exit code.

**Fix**: a gate arm that runs both files with the venv interpreter and fails on
skips, plus a CI step or an explicit statement in the README that these are
developer-run only.

---

### D3, a comment presented as measured is off by a factor of two

**Where**: `scripts/corpus_synth.py:90-93`.

```python
# The transfer-syntax cases are larger because OpenJPEG's default six
# resolution levels need at least 64 samples on the short side. Measured, not
# assumed: opj_start_compress fails outright below that.
SYNTAX_ROWS, SYNTAX_COLS = 64, 96
```

**What**: the threshold is 32, not 64. I bisected it against the same encoder
path the generator uses. `opj_start_compress()` fails at 31 and succeeds at 32.
Six resolution levels means five decompositions, so the arithmetic floor is
`2**5 = 32`, which agrees with the measurement and not with the comment.

**Why it is wrong**: a false factual claim, and one that explicitly invokes
measurement as its warrant. CLAUDE.md's review rule is that a comment "was
generated by the same process as the code and is not independent evidence", and
this is a live instance. The chosen value 64 is safe, so nothing is broken
today. The next person who needs a smaller transfer-syntax case will believe 64
is a floor when 32 is.

**Evidence**:

```
12x96 -> FAIL: failure result from 'opj_start_compress()'
16x96 -> FAIL
17x96 -> FAIL
20x96 -> FAIL
24x96 -> FAIL
31x96 -> FAIL
32x96 -> OK
33x96 -> OK
48x96 -> OK
64x96 -> OK
```

---

### D4, the NBIA licence-metadata claim is over-broad for EAY131

**Where**: `docs/SOURCE-POLICY.md`, "The four TCIA collections, against the
three questions", question 2.

> The NBIA REST service returns `LicenseName` and `LicenseURI` per series, and
> for all four collections above they are the Creative Commons Attribution 4.0
> International License at `https://creativecommons.org/licenses/by/4.0/`. Both
> sources were checked and they agree.

**What**: for EAY131 the service returns `null` for both fields on 15,799 of its
30,293 series. CMB-MML (1,156), VAREPOP-APOLLO (1,549) and CMB-CRC (2,537) are
uniformly CC BY 4.0. The unlicensed EAY131 series are all RTSTRUCT (14,395) and
SEG (1,404) third-party analysis objects, so the primary image series are fine,
but the sentence as written is a universal over four collections and it is false
for one of them.

**Why it is wrong**: microscope class 3, and it sits in the file the project
calls normative on provenance. The substance is sound and no redistribution
question is affected. The four series actually taken are all CC BY 4.0 by both
sources, which I verified independently.

**Evidence**:

```
CMB-MML          series= 1156  [('Creative Commons Attribution 4.0 International License', 'https://creativecommons.org/licenses/by/4.0/')]
VAREPOP-APOLLO   series= 1549  [(same)]
CMB-CRC          series= 2537  [(same)]
EAY131           series=30293  CC BY 4.0: 14494 | (None, None): 15799
                 null modalities: RTSTRUCT 14395, SEG 1404
```

**Fix**: scope the sentence to the image series taken, or to the primary image
series, and note that EAY131's third-party analysis series carry no licence
metadata.

---

## Smells

### S1, the real layer's only class-two case exercises no chroma, and nothing says so

`real/us_cmb_crc/00000001.dcm` is the single manifest row that satisfies
`coverage()`'s "the real layer has no colour or ultrasound case" condition. Read
independently, it is `SamplesPerPixel 1`, `PhotometricInterpretation
MONOCHROME2`, `BitsAllocated 8`, 590 by 819, GE Healthcare.

HLD 25.1 defines class two as "**Colour and ultrasound:** perceptual difference
below a stated threshold, **because chroma subsampling and YBR conversion
legitimately differ**". The stated reason for the class is chroma. A greyscale
8-bit ultrasound exercises none of it. The corpus's only YBR data is
`synthetic/us_ybr_full_422.dcm` and `syntax/jpeg_baseline_rgb8.dcm`, both
generated by this repository. So class two has never been diffed against a real
vendor's chroma, which is precisely the gap the "the real layer" arm of the
coverage check was written to close, and the check reports it closed.

Nothing in `corpus/README.md`, the manifest or `corpus_check.py` records this.
A reader of the coverage output, which prints `colour or ultrasound rows: 6`,
would reasonably assume otherwise. Separately, HLD 25.1 has no class for 8-bit
monochrome at all, and this file is silently absorbed into class two by
modality.

**Fix**: one line in the `corpus/README.md` real-layer table or in the row's
`category` tokens recording that the real class-two case is 8-bit monochrome,
so the chroma gap is visible rather than assumed away. A real colour or
YBR ultrasound series is the fuller answer and is a later addition.

### S2, a row with an empty `transfer_syntax` passes every check silently

`load()`'s non-empty requirement is `modality, category, source, licence,
licence_url`. `transfer_syntax` is not in it, and that omission is pre-existing.
What is new is `coverage()`'s

```python
unknown = sorted(present - set(REGISTRY_TRANSFER_SYNTAXES) - {""})
```

The `- {""}` discards the empty string, so a row declaring no transfer syntax is
neither counted for coverage nor named as unknown. It is simply invisible.

This is asymmetric with the treatment two blocks below, where a row declaring no
tolerance class is named per path with an explanation. Condition 4 of this story
is "at least one case per transfer syntax", and a row claiming none should be
named the same way.

It is reachable through the documented path. `corpus_check.py --add` defaults
`--transfer-syntax` to `""`, and `corpus/README.md`'s "Adding a case" step 2
shows `--add <file> --modality CT --category ...` with no `--transfer-syntax`,
warning only about `category`.

**Evidence**: appending

```
real/ghost.dcm  CT  (empty)  real, mono16  somewhere  CC BY 4.0  https://x  0000...  (empty)
```

gives `OK: coverage complete`, exit 0, and `OK: manifest shape valid, 92 rows`,
exit 0. Restored and confirmed with `diff -q`.

### S3, the determinism caveat names two encoders of at least three, and no version is recorded anywhere

Both `scripts/corpus_synth.py` and `corpus/README.md` say the same thing:

> a codestream carries whatever its encoder produced, and OpenJPH writes its own
> version into a comment marker. A different DCMTK or OpenJPH build can change a
> compressed case's digest.

OpenJPEG does it too. `j2k_lossless.dcm` and `j2k_lossy.dcm` carry a COM marker
reading `Created by OpenJPEG version 2.5.2`, exactly as the HTJ2K cases carry
`OpenJPH Ver 0.31.0`. So three of the four external encoder paths move the
digest on a version bump and the prose names two.

The larger half of this: nothing anywhere records the versions this manifest was
built against. `corpus/README.md`'s prerequisites list carries no version for
pydicom, pylibjpeg-openjpeg, pyjpegls, DCMTK or OpenJPH. A second developer
following the README to the letter will very likely get digest mismatches on the
eleven encoder-produced rows, and `corpus_check.py` will tell them "the thing the
tolerance policy is measured against has moved" with no way to distinguish a
toolchain bump from a corrupted corpus. For the story that gates the whole
programme, that is the difference between a reproducible manifest and one that
only this machine can reproduce.

This machine: DCMTK 3.7.0, OpenJPH 0.31.0, OpenJPEG 2.5.2 (pylibjpeg-openjpeg
2.5.0), pyjpegls 1.5.1, pydicom 3.0.2, numpy 2.5.2.

**Fix**: record the exact versions in `corpus/README.md` next to the
prerequisites, and add OpenJPEG to the sentence.

### S4, the fourteen mono16 syntax cases are one degenerate series

`normalise()` rewrites `SOPInstanceUID` per case but leaves `SeriesInstanceUID`,
`StudyInstanceUID` and `FrameOfReferenceUID` inherited from the base. Read back
from the written files, `syntax/` is one study containing three series, and the
mono16 series holds fourteen instances that share a frame of reference, all carry
`ImagePositionPatient [0.0, 0.0, 0.0]` and `InstanceNumber "1"`, and declare
fourteen different transfer syntaxes.

Every rule in the `dicom-expert` volume-construction section applies against it:
duplicate positions to reject, and a series that is not a volume. It is fine for
the per-instance codec tests it was built for, and it is a trap for the oracle,
which HLD section 11 describes as pushing "the same study through both stacks".
A consumer that groups `syntax/` by study or series gets fourteen co-located
images with mixed transfer syntaxes, which is not a case anyone intended to
create.

**Fix**: one line, derive the series UID from the case label in `normalise()`
the way the SOP UID already is.

---

## Nitpicks

**N1.** `test_real_rows_covering_only_one_class_fail` asserts only
`assertIn("real", text)`. The word `real` appears in the always-printed summary
line `coverage over 91 manifest rows, 44 of them real`, so that assertion is
satisfied by a passing run and proves nothing about the message naming the real
layer. The `assertEqual(status, 1)` beside it still binds, so the test is not
vacuous, only the naming half of it is.

**N2.** JPEG 2000 `.90` and `.91` are the only two syntaxes whose decode is
verified solely by the library that encoded them. No independent JPEG 2000
decoder is on this machine (`opj_decompress` is absent, `ojph_expand` refuses a
codestream without Rsiz bit 14). I verified their SIZ markers by hand instead,
96 by 64, one component, unsigned 16-bit, Rsiz 0x0000 and no CAP marker, which
correctly distinguishes them from the HTJ2K trio. Worth recording as the one
remaining single-tool link in the conformance chain.

**N3.** `encode_with_ojph` is handed `mono16_pixels`, recomputed by calling
`ramp()` again, rather than the pixels of the reference file whose header it
copies. Two sources of truth for one array. The lossless round-trip test would
catch a divergence, so it is only a maintenance hazard.

**N4.** Every one of the twelve encapsulated cases is one frame in one fragment
with a populated four-byte Basic Offset Table. The corpus therefore has no
multi-fragment case, no multi-frame encapsulated case and no empty-BOT case,
which are three named traps the codec story will meet. Outside this story's
declared scope, worth carrying into E2.6.

**N5.** The plan's fourth coverage bullet says `--coverage` restates the
licence and licence-url non-empty check "so the coverage report is a single
answer". It does not. `load()` enforces it and `coverage()` runs after `load()`,
so the property holds, but the coverage report is not the single answer the plan
described.

---

## What I checked and found correct

**Licences, which are the ones that cannot be fixed later.** All four TCIA
`LICENSE` files read in full. Each names its collection and CC BY 4.0 at
`https://creativecommons.org/licenses/by/4.0/`, matching the manifest rows
exactly. All four DOIs resolve with HTTP 200 to the right collection page:
`10.7937/SZKB-SW39` to cmb-mml, `10.7937/GHKN-MD15` to varepop-apollo,
`10.7937/c5ke-yx42` to EAY131, `10.7937/DJG7-GZ87` to cmb-crc. The NBIA API
reports CC BY 4.0 for each of the four exact `SeriesInstanceUID` values, with
`ImageCount` 27, 15, 1 and 1, matching the manifest row counts and the
`corpus/README.md` table. No disagreement with the integrator's table on any
cell. The synthetic rows' `MIT OR Apache-2.0` matches `LICENSE`,
`LICENSE-MIT`, `LICENSE-APACHE` and `Cargo.toml`. The SOURCE-POLICY extension
rows are in the same five-column form as the table above them and all carry
`2026-09-04`. The pydicom refusal is recorded with its reason and with the
verbatim sentence it turns on.

**Transfer syntaxes, independently of the generator.** A raw byte parse of the
file-meta group, using neither pydicom nor DCMTK, over all 91 rows: zero
mismatches against the `transfer_syntax` column. The manifest's `modality`
column and its `mono16` and `colour` class tokens also agree with the files, by
a second pass reading `Modality`, `SamplesPerPixel`, `BitsAllocated` and
`PhotometricInterpretation`. The README claim that the MR series is Implicit VR
Little Endian and the other three Explicit VR Little Endian is true.

**Decoding, with tools other than the ones that encoded.** `dcmdrle`,
`dcmdjpeg` and `dcmdjpls` decode the RLE, four JPEG and two JPEG-LS cases.
Compared against the uncompressed references: RLE, JPEG lossless process 14,
process 14 SV1 and JPEG-LS lossless are all bit-exact, max error 0. JPEG-LS
near-lossless has max error exactly 3, equal to the declared NEAR, which is the
ISO/IEC 14495-1 bound. JPEG Extended 12-bit max error 2, JPEG baseline RGB max
error 4. `ojph_expand` decodes the three HTJ2K cases: `.201` and `.202` are
bit-exact against the reference, `.203` lossy at max error 41 of 65535.

**The two fixes the worker reported.** Encapsulated `PixelData` is `OB` with
undefined length on all twelve compressed cases, and the four native ones are
`OW` with a defined length, which is what PS3.5 Table 7.1-1 and A.4 require. The
RLE header's first value is 2, offsets 64 and 576, which is PS3.5 Annex G's one
segment per byte of a 16-bit sample. Both verified from the bytes. Mutating away
the `OB` assignment turns three subtests red.

**Codestream identity, not just the header.** HTJ2K `.201` COD progression byte
0 (LRCP), `.202` and `.203` byte 2 (RPCL), all three with SIZ Rsiz 0x4000 and a
CAP marker. The two plain JPEG 2000 cases have Rsiz 0x0000 and no CAP, so the
pair is genuinely distinguishable and not merely differently labelled. All
twelve encapsulated cases have a well-formed BOT item plus one fragment, every
item length even.

**Determinism, checked by me rather than by its own test.** Two separate
interpreter processes into two fresh directories produce identical sha256 for
all 47 files. A third fresh generation matches all 47 committed manifest digests
exactly. The suite's own test really does spawn subprocesses via
`subprocess.run([sys.executable, ...])` and really does run the generator, which
takes 0.27s per run because the frames are tiny. Grep for `generate_uid`,
`datetime.now`, `date.today`, `time.time`, `random`, `uuid`, `os.urandom`,
`getpid` and bare `hash(` across the generator and the checker: no hits. A scan
of every `DA`, `TM` and `DT` element across all 47 files finds only
`StudyDate/ContentDate 20200101`, `StudyTime/ContentTime 120000.000000` and an
empty `PatientBirthDate`. `--write-manifest` is idempotent and keeps all 44 real
rows.

**The hand-computed fixture.** I computed the whole table myself from
`shift = HighBit + 1 - BitsStored`, `mask = (1 << BitsStored) - 1` and sign
extension from bit `BitsStored - 1`. Both `RIGHT_ALIGNED` and `LEFT_ALIGNED`
agree with my table and with the integrator's on all sixteen cells, including
`0xF800` to -2048 and -128, `0x0FFF` to -1 and 255, and `0x800F` to 15 and
-2048. The worked derivations in the comment are correct line by line. The
expected values are not read back from the generator, and the generator contains
no unpacking code they could have come from.

**Geometry.** `row (0.8, 0.6, 0)` cross `col (0, 0, -1)` is `(-0.6, 0.8, 0)`,
which is `SERIES_NORMAL`. Projected IPP onto that normal reproduces
`index * 2.5` exactly. The non-uniform series has gaps 3.75 and 1.25 either side
of slice 7 with a median of 2.5, and the test asserts all three. Spacing is
derived from projected IPP, and `SpacingBetweenSlices` is deliberately absent.

**YBR_FULL_422, against the standard rather than the comment.** PS3.3
C.7.6.3.1.2 requires two Y values followed by one CB and one CR, requires
`PlanarConfiguration` 0, and gives a frame length of Rows times Columns times 2.
The generator writes exactly that and the test asserts exactly that.

**`LossyImageCompressionMethod ISO_15444_15`, settled.** It is a Defined Term in
PS3.3 C.7.6.1.1.5.1, "High-Throughput JPEG 2000 Irreversible Compression",
alongside ISO_10918_1, ISO_14495_1, ISO_15444_1, ISO_18181_1, ISO_13818_2,
ISO_14496_10 and ISO_23008_2. The worker's inference was correct. Also correct:
`ISO_14495_1` on the JPEG-LS near-lossless case, `ISO_15444_1` on JPEG 2000
lossy, `ISO_10918_1` on the two DCTs, and `LossyImageCompression` absent from
every lossless case.

**`REGISTRY_TRANSFER_SYNTAXES`.** Exactly the sixteen PS3.5 Annex A UIDs of the
plan's table, no additions, no omissions, no duplicates, and in the plan's
order. The claim in the comment that `docs/hld/18-codec-registry.md` carries no
list of syntaxes is true, that file is HLD section 21 and contains zero
occurrences of `1.2.840.10008`. A1 is the HTJ2K gate and A2 the JPEG-LS gate per
`docs/hld/A-spike-gates.md`.

**The coverage guard, made to fail three ways.** Header-only manifest: exit 1,
all sixteen syntaxes named plus both classes and the synthetic-only condition.
Only RLE row removed: exit 1 naming `1.2.840.10008.1.2.5`. RLE row's syntax
changed to `1.2.840.10008.1.2.4.100`: exit 1 naming both the missing UID and the
unknown one. Manifest restored and proved with `diff -q` after each.

**Gate chaining.** With coverage broken and digests intact, `bin/ocelli.sh gate
corpus` exits 1 and prints `FAILED corpus`. The `&&` is doing its job and the
trap the `backlog` arm's comment describes is avoided.

**Prose scope.** `scripts/prose_check.py` covers both `corpus/README.md` and
`docs/SOURCE-POLICY.md` by exact name, `gate prose` is green over 44 files, and
a direct grep finds no em-dash in any file this story touched.

**Acquisition procedure.** The four `SeriesInstanceUID` values in
`corpus/README.md` each match the single series present in the corresponding
corpus directory. The `--add` example reproduces the manifest row shape shown,
verified end to end against a temporary extra file, then removed. Every real row
has an empty `url` as the README says, and the `LICENSE` files carry no manifest
row as the README says.

**Gates run.** `gate corpus content provenance prose` all green.
`gate --floor` green, 12 passed 4 skipped (docs, lint, types, wasm, all for
pre-existing reasons). The full 39-test suite green.

**Working tree.** Restored to the eight-file diff it started as, confirmed by
`git status --short` and an unchanged `git diff --stat`.

---

## Mutations run, and what went red

Each was applied to the working tree, the suite run, then reverted from a copy
taken beforehand and the revert proved with `diff -q`.

| # | Mutation | Result |
|---|----------|--------|
| M1 | `HighBit` 11 to 10 on `ct_signed_12in16_right` | RED, 1 failure and 1 error |
| M2 | `PixelRepresentation` 1 to 0 on both signed cases | RED, 4 failures |
| M3 | `NON_SQUARE_SPACING` `["0.5", "0.25"]` to `["0.5", "0.5"]` | RED, `test_pixel_spacing_is_non_square_where_it_should_be` |
| M4 | `ct_common` sets `AcquisitionDate` from `date.today()` | RED, 39 failures, `test_no_case_carries_a_clock_reading` on every case |
| M5 | remove `ds["PixelData"].VR = "OB"` from `encode_with_ojph` | RED, 3 subtests of the encapsulation test |
| M6 | write the `.202` case LRCP instead of RPCL | RED, `test_htj2k_progression_order_matches_the_syntax` |
| M7 | drop the big-endian pixel byte swap | RED, `test_lossless_syntaxes_round_trip_exactly` on `explicit_vr_be.dcm` |
| M8 | header-only manifest | `--coverage` RED, exit 1 |
| M9 | drop the only RLE manifest row | `--coverage` RED naming the UID, and `gate corpus` RED |
| M10 | manifest syntax not in the registry | `--coverage` RED naming both problems |
| M11 | manifest row with an empty `transfer_syntax` | **GREEN**, which is smell S2 |

Every mutation except M11 went red. M11 is the finding.
