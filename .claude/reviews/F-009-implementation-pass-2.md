# F-009, implementation review, pass 2

**Reviewer**: independent agent, wrote neither the work nor pass 1
**Diff reviewed**: working tree, base cd74768
**Result**: 3 defects, 2 smells, 4 nitpicks

The hard rule first, because it outranks everything else. A Python walk over
every file in the worktree outside `.git`, including the four ignored trees
(`.claude/scratch/`, `scripts/__pycache__/`, `scripts/tests/__pycache__/`,
`target/`), reading bytes 128 to 131 of each of 1,604 files and separately
matching `.dcm`, `.dicom` and `.ima` by suffix: **zero hits**. No DICOM is in
this repository. The corpus is entirely under `$OCELLI_CORPUS_DIR` and all 91
manifest rows verify there.

Every finding below was reproduced by running something. Every mutation was
reverted and the revert proved with `diff -q`. The working tree at the end of
this pass is byte-identical to the copy taken before it started, `.claude/`
included, except for this file.

---

## Pass 1 findings, and whether each is genuinely resolved

### D1, `--coverage` is not run by CI. **Resolved.**

`.github/workflows/ci.yml:108` is now `python3 scripts/corpus_check.py
--coverage` in the `guards` job. I confirmed the subsumption claim rather than
reading it: with a two-field row appended to the manifest, **both**
`--coverage` and `--manifest-only` exit 1 with `manifest line 93 has 2 fields,
expected 9`, because `main()` calls `load()` before dispatching on either flag.
The `--floor` exclusion of `corpus` is untouched, `case "$name" in
oracle|corpus) continue` at `bin/ocelli.sh:160`, with the shell case terminator following it.

The `corpus` arm is now `--coverage && corpus_check.py`. Chaining verified by
breaking coverage without breaking any digest (stripped the class token from
`synthetic/ct_unsigned_16.dcm`): `bin/ocelli.sh gate corpus` exits 1, prints
`FAILED corpus`, and names the row.

### D2, nothing executes the Python tests. **Resolved, with one new false count (defect D2 below).**

`scripts/corpus_tests.py` + the `corpus-tests` gate arm + the `corpus-tooling`
CI job. Each of the four sub-claims verified by doing:

- **Fails on a skip, not on the exit code.** Decorated
  `test_ybr_full_422_frame_is_two_bytes_per_pixel` with `@unittest.skip`. Plain
  `unittest discover` under the venv interpreter exits **0**. `bin/ocelli.sh
  gate corpus-tests` exits **1**, prints `OCELLI-SKIP
  test_ybr_full_422_frame_is_two_bytes_per_pixel ... :: MICROSCOPE-PASS2` and
  `1 test(s) SKIPPED. A skipped test is not a passed test`. Reverted.
- **The interpreter is resolved and the explicit sources are authoritative.**
  `OCELLI_PYTHON=/usr/bin/python3` (no pydicom) gives
  `OCELLI_PYTHON was set explicitly, so no fallback was tried. Fix it or unset
  it.` and does **not** silently run the working venv. Same for
  `OCELLI_PYTHON=/nonexistent/python`. The generator suite is then reported
  SKIPPED and the runner exits 3, which the gate names as a skip and never as a
  pass. It reports a skip rather than a hard failure, which is a defensible
  reading of the docstring's "one skip that is allowed", so I am not calling it
  a defect. See nitpick N3 for the one place the prose is broader than the
  behaviour.
- **A missing external tool is a named skip.** `PATH=/usr/bin:/bin` strips
  `dcmcjpeg` and `ojph_compress`: `test_corpus_synth.py: SKIPPED, not on PATH:
  dcmcjpeg (DCMTK, ...), ojph_compress (OpenJPH, ...)`, exit 3. Never a pass.
- **The floor placement is right.** `corpus-tests` needs no corpus. Proved
  twice: `OCELLI_CORPUS_DIR=/nonexistent/corpus bin/ocelli.sh gate
  corpus-tests` and the same with the variable unset both go ALL GREEN, 44
  tests. `test_corpus_synth` generates into its own
  `tempfile.TemporaryDirectory`, `test_corpus_check` swaps
  `corpus_check.MANIFEST` to a temporary file. `bin/ocelli.sh gate --floor`
  reports **13 passed, 4 skipped**, as claimed.
- **The CI job.** Read critically below. Two problems found, defect D3 and
  smell S2.

### D3, the OpenJPEG size comment. **Resolved, and I re-bisected it.**

Encoding a 16-bit mono ramp through the same `ds.compress(1.2.840.10008.1.2.4.90)`
path the generator uses, short side varied against a fixed 96:

```
  8x96 FAIL   16x96 FAIL   24x96 FAIL   28x96 FAIL   30x96 FAIL   31x96 FAIL
 32x96 OK     33x96 OK     48x96 OK     64x96 OK
```

The raw error through `openjpeg.utils.encode_buffer` at 31 is
`RuntimeError: Error encoding the data: failure result from
'opj_start_compress()'`, verbatim what the comment now claims. Six resolution
levels is five decompositions, floor `2**5 = 32`, and 64 is that doubled. Every
clause of the replacement comment at `scripts/corpus_synth.py:98-107` is true.

### D4, the NBIA licence claim. **Resolved, and I re-verified the whole thing, not a spot check.**

`getSeriesMetaData` for each of the four `SeriesInstanceUID` values in
`corpus/README.md` returns `Creative Commons Attribution 4.0 International
License` / `https://creativecommons.org/licenses/by/4.0/`, with collections
CMB-MML/EAY131/VAREPOP-APOLLO/CMB-CRC and modalities CT/MR/DX/US matching the
README table cell for cell. The aggregate figures in the new SOURCE-POLICY
paragraph are exact: CMB-MML 1,156 series all CC BY 4.0, VAREPOP-APOLLO 1,549
all, CMB-CRC 2,537 all, and EAY131 30,293 total, 14,494 CC BY 4.0 and 15,799 with
both fields null, those null ones being exactly RTSTRUCT 14,395 and SEG 1,404.
All four DOIs still resolve 302 to the right collection page.

### S1, the chroma gap. **Resolved.**

Three places, all present, and the note is genuinely conditional rather than
unconditional:

- the manifest row carries `greyscale-8bit, chroma-untested`
- `corpus/README.md` has the real-layer table with **no chroma** on the US row
  plus a paragraph naming the gap
- `--coverage` prints `colour or ultrasound rows: 6 (1 real, of which 0 carry
  chroma)` and then the NOTE

The NOTE is advisory and **cannot** fail the check: with the note printing,
`--coverage` exits 0. Adding a `colour` token to the real US row removes the
note and still exits 0. `if real_class_two and not real_chroma:` mutated to
`if True:` turns `test_a_real_colour_row_settles_the_chroma_note` red, so the
condition is load-bearing and the negative assertion is not vacuous (it keys on
the sentence, not on the word "chroma", which also appears in the always
printed counts line).

### S2, an empty `transfer_syntax`. **Resolved.**

Pass 1's M11 reproduced exactly: appending
`real/ghost.dcm  CT  (empty)  real, mono16  ...` now gives

```
FAIL: corpus coverage
  real/ghost.dcm: declares no transfer syntax. The column is what condition 4
  is counted from, ...
```

exit 1, named per path. `--manifest-only` still exits 0 on it, which is
correct, that mode is a shape check. Reverted and proved.

### S3, encoder versions. **Partly resolved. The version table is right, the prose around it is not. See defect D1.**

`--tool-versions` runs and reports all seven rows matching, exit 0. Faking a
DCMTK drift in `BUILT_WITH` makes it print `<- differs` and exit **1**, so it
fails rather than warns. Every version it asserts agrees with
`corpus/README.md`'s prerequisites table, checked cell by cell. That half is
sound. The sentences explaining it are not.

### S4, the degenerate syntax series. **Resolved.**

Read back from the 18 files in the real corpus's `syntax/` directory, not from
the generator: **18 distinct `SeriesInstanceUID`, 18 distinct
`FrameOfReferenceUID`, 1 `StudyInstanceUID`**, no UID shared by two files.
Both new tests can fail: deleting the two lines from `normalise()` turns
`test_each_case_is_its_own_series_and_spatial_frame` red on both subtests, and
replacing `study="syntax"` with `study=label` turns `test_they_stay_one_study`
red.

---

## New defects

### D1, the encoder-version prose is false in three ways, in six places

**Where**: `corpus/README.md:80` and `:118-123`, `scripts/corpus_synth.py:27-31`,
`:703-705`, `:765`, `:781`.

**What**, and each part checked against the bytes:

1. `corpus/README.md:118-121` says "**Three external encoders are involved and
   all three stamp a version into their output.** OpenJPH writes `OpenJPH Ver
   0.31.0` ... OpenJPEG writes `Created by OpenJPEG version 2.5.2` ... and
   DCMTK records its quality factor and compression ratio in the
   `DerivationDescription`". `scripts/corpus_synth.py:27-29` says the same,
   "THREE of them are involved and all three stamp a version". **DCMTK stamps
   no version anywhere.** Its `DerivationDescription` on `jpeg_lossless_p14.dcm`
   reads, in full, `Lossless JPEG compression, selection value 6, point
   transform 0, compression ratio 1.4487`. There is no version in it, and a
   byte search for `DCMTK` or `OFFIS` across all four DCMTK-produced files
   returns nothing. Only OpenJPH and OpenJPEG stamp a version, on five rows of
   the eleven.

2. Same sentence: "in the `DerivationDescription` of **the two lossy JPEG
   cases**". **All four** DCMTK cases carry a `DerivationDescription`. The two
   lossless ones record selection value, point transform and compression ratio.
   the two lossy ones additionally record the IJG quality factor.

3. `corpus/README.md:122` and `scripts/corpus_synth.py:30`, `:781`: "A version
   bump in DCMTK, OpenJPH or OpenJPEG changes the digest of **the eleven**
   encoder-produced rows." Those three tools own **nine** of the eleven, 4 + 3 +
   2. The other two are `jpegls_lossless.dcm` and `jpegls_near_lossless.dcm`,
   produced by pyjpegls, which is not one of the three and whose codestreams
   carry no version stamp at all. The count eleven is itself correct (RLE goes
   through pydicom's own native encoder, `RLELosslessEncoder.available_plugins`
   is `('pydicom', 'pylibjpeg')` so the first is chosen), but it is only eleven
   because it counts the two pyjpegls rows the sentence then attributes to the
   three named tools.

**Why it is wrong**: microscope class 3. Pass 1's S3 was that the prose named
two encoders of at least three. The remediation went to three and left out the
fourth, while quietly borrowing the fourth's two rows to reach its count. The
machinery is fine, `BUILT_WITH` and the README table both carry pyjpegls 1.5.1
and `--tool-versions` checks it, so nothing is broken today. What is broken is
that a developer whose JPEG-LS digests move is told by three separate documents
to look at DCMTK, OpenJPH and OpenJPEG, which are the three tools that did not
cause it.

**Evidence**:

```
$ python - <<'EOF'   # byte search over corpus/syntax/*.dcm
htj2k_lossless.dcm       OpenJPH-COM=OpenJPH Ver 0.31.0.
htj2k_lossless_rpcl.dcm  OpenJPH-COM=OpenJPH Ver 0.31.0.
htj2k_lossy.dcm          OpenJPH-COM=OpenJPH Ver 0.31.0.
j2k_lossless.dcm         OpenJPEG-COM=Created by OpenJPEG version 2.5.2
j2k_lossy.dcm            OpenJPEG-COM=Created by OpenJPEG version 2.5.2
(every other file, including all four DCMTK cases and both JPEG-LS cases: -)

$ # DerivationDescription, read with pydicom
jpeg_lossless_p14.dcm      'Lossless JPEG compression, selection value 6, point transform 0, compression ratio 1.4487'
jpeg_lossless_p14_sv1.dcm  'Lossless JPEG compression, selection value 1, point transform 0, compression ratio 3.0937'
jpeg_baseline_rgb8.dcm     'Lossy compression with JPEG baseline, IJG quality factor 90, compression ratio 19.692'
jpeg_extended_12.dcm       'Lossy compression with JPEG extended sequential 12 bit, IJG quality factor 90, compression ratio 12.063'
j2k_lossy.dcm              None
jpegls_near_lossless.dcm   None

$ # first 400 bytes of each codestream, printable runs
jpegls_lossless.dcm       len=  1354 first8=ffd8fff7000b1000 strings=[b'DBCCABBABD', ...]
jpegls_near_lossless.dcm  len=  1052 first8=ffd8fff7000b1000 strings=[b"I'UUUUU"]
```

**Fix**: name the fourth encoder, and split the sentence. Nine rows move on
DCMTK, OpenJPH or OpenJPEG, two more move on pyjpegls, five of the eleven carry
a readable version and the other six do not. Or delete the enumeration and keep
only "eleven rows are external encoder output, `--tool-versions` says which
tools moved", which is the sentence that is both short and true.

### D2, `scripts/corpus_tests.py` says the suite has 39 tests. It has 44.

**Where**: `scripts/corpus_tests.py:5`.

> Without this runner the 39 Python tests under `scripts/tests/` execute when
> somebody remembers the command

**What**: 15 in `test_corpus_check.py` and 29 in `test_corpus_synth.py`, 44.
The number 39 is pass 1's count, taken before this round added five tests to
`test_corpus_check.py` (the blank-transfer-syntax test, the two chroma-note
tests and the two `SyntaxSeriesShape` tests, which is +2 and +3 across the two
files) and the file that states it is new in this round.

**Why it is wrong**: microscope class 3, and this story's declared repeat
offence. The runner's own output prints `ran=15` and `ran=29` on every
invocation, so the docstring is contradicted by the program it documents,
every time it runs.

**Evidence**:

```
$ bin/ocelli.sh gate corpus-tests 2>&1 | grep OCELLI-SUITE
OCELLI-SUITE test_corpus_check.py ran=15 failures=0 errors=0 skipped=0
OCELLI-SUITE test_corpus_synth.py ran=29 failures=0 errors=0 skipped=0
```

**Fix**: delete the number. "the Python tests under `scripts/tests/`" says
everything the sentence needs to say and cannot go stale.

### D3, the CI job says its versions are pinned and must agree with `corpus/README.md`. DCMTK is neither.

**Where**: `.github/workflows/ci.yml:119-122` and `:130-131`.

```yaml
      # The generator writes real DICOM in sixteen transfer syntaxes, so it
      # needs real encoders. Versions are pinned because eleven manifest rows
      # are encoder output and a version bump moves their digests. The set and
      # the versions are documented in corpus/README.md, and they must agree.
      ...
      - name: DCMTK, for the four JPEG syntaxes pydicom cannot encode
        run: sudo apt-get update && sudo apt-get install -y dcmtk
```

**What**: every Python package is pinned with `==` and OpenJPH is pinned to the
tag `0.31.0`, and I confirmed all eight of those pins are real and installable
on Python 3.12. DCMTK is not pinned at all. `ubuntu-latest` is Ubuntu 24.04,
whose `dcmtk` binary package is **3.6.7-9.1build4**. `corpus/README.md`'s
prerequisites table and `corpus_synth.BUILT_WITH` both record **3.7.0**, which
is what this machine has. They cannot agree, and the comment asserting they
must is the only thing that would tell a reader otherwise.

**Why it is wrong**: microscope class 3, in the one file the reviewer cannot
execute, which is where a false claim survives longest. It also leaves the
whole point of the S3 remediation unenforced in CI: `--tool-versions` is the
mechanism that catches this and no job runs it, and `corpus-tests` compares no
digest against the manifest, so the `corpus-tooling` job will run and pass
happily against a DCMTK that would regenerate four different digests.

**Evidence**:

```
$ curl -s https://packages.ubuntu.com/noble/dcmtk | grep -oE "Package: dcmtk \([^)]*\)"
Package: dcmtk (3.6.7-9.1build4)
$ dcmcjpeg --version | head -1        # this machine, and corpus/README.md's table
$dcmtk: dcmcjpeg v3.7.0 2025-12-15 $
$ git ls-remote --tags https://github.com/aous72/OpenJPH.git | grep -c "refs/tags/0.31.0$"
1                                      # the OpenJPH pin, by contrast, is real
```

Every pinned Python version exists on PyPI with a cp312 wheel or a pure-Python
wheel: numpy 2.5.2, pydicom 3.0.2, pyjpegls 1.5.1, pylibjpeg-openjpeg 2.5.0,
pylibjpeg 2.1.0, pylibjpeg-libjpeg 2.4.0, pylibjpeg-rle 2.2.0. So the claim is
true for seven of the eight prerequisites and false for the one that is
installed from a distribution.

**Fix**: either build DCMTK from the 3.7.0 tag the way OpenJPH is built, or
correct the comment to say DCMTK comes from the distribution unpinned and
therefore does not match, and add a `--tool-versions` step so the mismatch is
reported rather than assumed away. The second is cheaper and is honest.

---

## New smells

### S1, `.claude/commands/verify.md` counts the gates and the count is now wrong under every reading

**Where**: `.claude/commands/verify.md:27`, "Seventeen gates."

This diff adds `corpus-tests` to the `GATES` array. `bin/ocelli.sh gate --list`
now prints **19** rows, 18 of them non-GPU. At cd74768 it printed 18, 17 of
them non-GPU, so the sentence was already wrong against the list it sits
directly beneath and right against the non-GPU subset. After this diff it is
wrong against both.

I am calling this a smell rather than a defect because the sentence is
pre-existing and is not in the diff. It is here because F-009 is the change
that touched the gate table, because nothing checks the number, and because the
paragraph's own argument, "this file does not duplicate the list, because two
lists drift and then nobody knows which is the gate", is an argument against the
number being there at all.

**Fix**: delete "Seventeen". The next sentence already says `bin/ocelli.sh` is
the definition.

### S2, the `corpus-tooling` CI job has no assertion that the generator suite actually ran

**Where**: `.github/workflows/ci.yml:146`, the single `- run: bin/ocelli.sh
gate corpus-tests`.

`run_gate` returning 3 is a skip, and `gates_cmd` returns **0** when there are
skips and no failures. So if `ojph_compress` or `dcmcjpeg` is not on `PATH`
after the install steps, or the runner's Python cannot import pydicom, the job
prints `GREEN 0 passed, 1 skipped. A skipped gate is NOT a pass.` and **exits
0**. GitHub Actions reads the exit code, so the job is a green tick and the
hand-computed PS3.3 fixture, the determinism proof and all twelve conformance
assertions did not run.

That is the correct behaviour for `docs` and `wasm`, whose skips are permanent
and expected. It is not correct here: this job's four earlier steps exist
precisely to make those prerequisites present, so a skip in this job means an
install silently half-succeeded, which is a condition that should be loud. The
skip path is reachable in ways an install step would not catch, for example an
OpenJPH build that configures with `OJPH_BUILD_EXECUTABLES` off or installs to
a prefix outside `PATH`.

**Evidence**, locally, by removing the prerequisite the guard protects:

```
$ env PATH=/usr/bin:/bin:/usr/sbin:/sbin /usr/bin/python3 scripts/corpus_tests.py; echo $?
  test_corpus_check.py: 15 passed.
  test_corpus_synth.py: SKIPPED, not on PATH: dcmcjpeg (...), ojph_compress (...)
SKIPPED: a prerequisite is absent and is named above. This exits 3, ...
3
$ env PATH=/usr/bin:/bin bin/ocelli.sh gate corpus-tests >/dev/null 2>&1; echo $?
0                                  # the gate runner converts the skip to success
```

**Fix**: one line in the job, `python3 scripts/corpus_tests.py` directly rather
than through the gate runner, since exit 3 is then the job's exit and the job
goes red. Or keep the gate call and add a step that asserts both tools are on
`PATH` before it. The skip semantics are right for a developer's `gate --floor`
and wrong for the job whose whole purpose is to remove the reason to skip.

---

## Nitpicks

**N1.** `corpus_synth.py --tool-versions` raises `FileNotFoundError: 'dcmcjpeg'`
and prints a traceback when DCMTK is absent, where a missing Python package is
reported as `absent` in the table. The exit status is non-zero so it is not
silent, but the report the user ran the command for is not printed at all, and
this is the command the README tells a second developer to run first when their
digests do not match, which is also the moment they are most likely to be
missing a tool.

**N2.** The `chroma-untested` manifest token is inert. `--coverage`'s NOTE keys
on the absence of a `colour` token, not on that token, so if a real colour case
is ever added the note correctly disappears and `chroma-untested` stays on
`real/us_cmb_crc/00000001.dcm`, stale and unchecked by anything. Consistent
with `burned-in-unchecked`, which is also documentary, so this is convention
rather than a fault.

**N3.** `corpus/README.md` says of `$OCELLI_PYTHON` and `.ocelli-python-path`,
"if one is set and cannot import pydicom, no fallback is tried". A fallback is
tried for the stdlib suite: with `OCELLI_PYTHON=/nonexistent/python` the runner
still executes `test_corpus_check.py` under `sys.executable` and reports `15
passed`. Harmless, because that suite is stdlib-only and `main()` correctly
omits the `interpreter:` line in that case so nothing false is claimed about
which interpreter ran. The sentence is just broader than the code.

**N4.** `corpus/README.md` says the note prints "whenever no real class-two row
carries a `colour` token". The code is `if real_class_two and not real_chroma`,
so it is also suppressed when there are no real class-two rows at all. Coverage
fails in that case for a different and louder reason, so nothing is hidden.

---

## What I could not verify, and why

- **The `corpus-tooling` job actually executing in GitHub Actions.** I have no
  runner. The YAML parses as a mapping with five jobs and the new job's steps
  read correctly, the OpenJPH tag `0.31.0` exists, and `OJPH_BUILD_EXECUTABLES`
  defaults ON in that tag's `CMakeLists.txt` so `ojph_compress` should be built
  and installed to `/usr/local/bin`. Every pip pin resolves on PyPI with a
  cp312 wheel, and `cmake` is preinstalled on `ubuntu-latest`. What I cannot say is
  whether the OpenJPH configure step succeeds, because
  `OJPH_ENABLE_TIFF_SUPPORT` also defaults ON and the job installs no libtiff
  development package. If that turns out to be a hard requirement the step
  fails loudly, which is the acceptable direction, but it is unverified.
- **That a pyjpegls or DCMTK version bump would in fact move a digest.** I have
  one version of each. The direction of the argument is sound and the manifest
  digests are reproducible on this toolchain, which is what can be shown here.
- **`docs/SOURCE-POLICY.md`'s "Collections under Attribution-NonCommercial were
  available and were not taken."** True in the general sense that TCIA hosts
  CC BY-NC collections, but I did not enumerate what was considered, and it is
  a claim about a decision rather than about a file.
- **HLD section 25.1's wording** I read from `docs/hld/22-testing-and-tolerance.md`
  and it says exactly what the code and the README quote it as saying. I did
  not check it against the source `.docx`, which lives outside this repository.

---

## What I checked and found correct

**The hard rule.** 1,604 files scanned by magic bytes at offset 128 and by
suffix, ignored trees included. Zero DICOM. `gate content` green.

**The hand-computed fixture, recomputed from the standard.** I derived all
sixteen cells myself from `shift = HighBit + 1 - BitsStored`, `mask = (1 <<
BitsStored) - 1` and sign extension from bit `BitsStored - 1`, per PS3.3
C.7.6.3.1.4 and PS3.5 8.1.1, without looking at the table while doing it.
`RIGHT_ALIGNED` (shift 0): `0xF800 -> -2048`, `0x07FF -> 2047`, `0x0FFF -> -1`,
`0x0801 -> -2047`, `0x8000 -> 0`, `0x7FF0 -> -16`, `0xFFF0 -> -16`, `0x800F ->
15`. `LEFT_ALIGNED` (shift 4): `-128, 127, 255, 128, -2048, 2047, -1, -2048`.
Both agree with the file on every cell, and every worked derivation in the
comment block is correct line by line. The expected values are not read back
from the generator and the generator contains no unpacking code they could have
come from.

**Every file carries the transfer syntax its row claims.** A raw byte parse of
the file-meta group, no pydicom and no DCMTK, walking element headers from
offset 132 and reading (0002,0010): 91 rows checked, **0 mismatches**.

**Determinism, by my own two-process regeneration.** Two separate interpreter
processes into two fresh directories: 47 files each, identical names, **zero
digest differences**. All 47 match the committed manifest digests exactly. No
extra file generated that the manifest does not carry, and no manifest row
under `synthetic/` or `syntax/` that was not generated.

**Licences, the ones that cannot be fixed later.** All four `LICENSE` files
read: each names its own collection and CC BY 4.0 at the exact URL the manifest
carries. All 44 real rows carry `CC BY 4.0` and
`https://creativecommons.org/licenses/by/4.0/`, all with an empty `url` as the
README says, and no `LICENSE` file has a manifest row. Row counts per directory
27/15/1/1, matching the README table and the NBIA image counts. The four DOIs
resolve. The NBIA figures in SOURCE-POLICY are exact to the unit, see the D4
section above. The `pydicom` refusal quotes `test_files/README.txt:11`
verbatim, `I believe there is no restriction on using any of these files in
this manner.`, and the claim that the file traces individual files to several
upstream sources with differing terms is true (NEMA WG04 FTP, GDCM,
dclunie.com, barre.nom.fr, dcmtk.org, IHE MESA, dcmqi).

**The real-layer table, read out of the files.** `ct_cmb_mml` 27 files, CT,
Explicit VR LE, BitsAllocated 16 BitsStored 16 PixelRepresentation 1
MONOCHROME2, GE MEDICAL SYSTEMS. `mr_eay131` 15 files, MR, **Implicit VR LE**,
12-in-16 unsigned MONOCHROME2, SIEMENS. `dx_varepop` 1 file, DX, Explicit VR
LE, 12-in-16 MONOCHROME2. `us_cmb_crc` 1 file, US, Explicit VR LE, 8-bit
MONOCHROME2, SamplesPerPixel 1, no chroma. Every one of those cells matches the
README table, and each series is homogeneous (one distinct pixel-module and
transfer-syntax combination per directory).

**The JPEG 2000 SIZ claim in the README's gap list.** Parsed by hand from the
codestreams: `j2k_lossless` and `j2k_lossy` both `Rsiz 0x0000`, `Xsiz 96`,
`Ysiz 64`, `Csiz 1`, `Ssiz 0x0f` (16-bit unsigned), no CAP marker. The HTJ2K
trio has `Rsiz 0x4000` and a CAP marker. Exactly what the README says, and the
pair really is distinguishable rather than differently labelled. No independent
JPEG 2000 decoder is on this machine (`opj_decompress`, `opj_dump`,
`kdu_expand`, `gdcmconv` all absent), which is why the README names that as a
remaining gap and is right to.

**`REGISTRY_TRANSFER_SYNTAXES`.** Sixteen PS3.5 Annex A UIDs, no duplicates,
and `test_the_registry_list_is_the_codec_registry_table` types them in
independently rather than reading them back from the module. The corpus covers
all sixteen, read out of the written files by
`test_every_registry_syntax_has_a_case`.

**The coverage guard, made to fail four ways.** Blank transfer syntax, a
non-registry UID (names both the missing `1.2.840.10008.1.2.5` and the unknown
`1.2.840.10008.1.2.4.100`), a row with no tolerance class, and a malformed row.
All exit 1 and all name the row or the UID. Manifest restored and proved with
`diff -q` after every one.

**Gates.** `gate corpus corpus-tests content provenance prose` ALL GREEN, 5
gates. `gate --floor` GREEN, 13 passed 4 skipped (docs, lint, types, wasm, all
for pre-existing reasons). `corpus_check.py` in all three modes green over the
real corpus, 91 verified 0 missing 0 mismatched. The full 44-test suite green.

**Prose rules.** No em-dash or en-dash anywhere in the ten files this diff
touches, checked by direct grep, and no prose semicolon in the two Markdown
files. `gate prose` green over 45 files and its file list covers both
`corpus/README.md` and `docs/SOURCE-POLICY.md` by exact name.

**Working tree.** `diff -rq` against the copy taken before this pass, `.claude/`
included: identical. `git status --short` is the same twelve-entry list it
started as.

---

## Mutations run, and what went red

Each applied to the working tree, the affected suite or check run, then
reverted from a copy taken beforehand and the revert proved with `diff -q`.

| # | Mutation | Result |
|---|----------|--------|
| M-A | manifest row with 2 fields | `--coverage` **RED** exit 1, same message as `--manifest-only`. D1's subsumption claim |
| M-B | manifest row with an empty `transfer_syntax` (pass 1's M11) | `--coverage` **RED** exit 1, named per path. S2 fixed |
| M-C | give the real US row a `colour` token | note vanishes, still exit 0. The NOTE is conditional |
| M-D | `HighBit` 11 to 10 on `ct_signed_12in16_right` | **RED**, 1 failure and 1 error |
| M-E | `PixelRepresentation` 1 to 0 on both signed cases | **RED**, 4 failures |
| M-F | `NON_SQUARE_SPACING` to `["0.5", "0.5"]` | **RED**, `test_pixel_spacing_is_non_square_where_it_should_be` |
| M-G | delete the two S4 lines from `normalise()` | **RED**, both subtests of `test_each_case_is_its_own_series_and_spatial_frame` |
| M-H | `study="syntax"` to `study=label` | **RED**, `test_they_stay_one_study` |
| M-I | chroma NOTE condition to `if True` | **RED**, `test_a_real_colour_row_settles_the_chroma_note` |
| M-J | strip the class token from one row, digests intact | `gate corpus` **RED** exit 1, so the `&&` chain works |
| M-K | RLE row's syntax to `1.2.840.10008.1.2.4.100` | `--coverage` **RED**, names the missing UID and the unknown one |
| M-L | fake a DCMTK drift in `BUILT_WITH` | `--tool-versions` **RED** exit 1, prints `<- differs`. It fails, not warns |
| M-N | `@unittest.skip` on one generator test | plain `unittest discover` exit **0**, `gate corpus-tests` exit **1** naming the test. D2's central claim |
| M-O | `OCELLI_PYTHON=/usr/bin/python3` | no fallback, generator suite SKIPPED, exit 3, never a pass |
| M-P | `PATH` stripped of `dcmcjpeg` and `ojph_compress` | named skip, exit 3, never a pass |
| M-Q | `OCELLI_CORPUS_DIR` bogus, then unset | `gate corpus-tests` ALL GREEN both times. The floor placement is right |

Every mutation went the direction it should. The three defects came from
reading files against the bytes and against the distribution, not from a
mutation.
