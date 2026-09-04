# The golden corpus

The corpus is **not in git**, deliberately. `corpus/manifest.tsv` is, and it is
what makes the corpus verifiable without being present.

## Where it lives

```bash
export OCELLI_CORPUS_DIR=/path/to/your/corpus
python3 scripts/corpus_check.py            # verify against the manifest
```

Default when the variable is unset: `corpus/data/`, which is gitignored.

## Why it is not committed

Three reasons, in order of how much they bind:

1. **Redistribution terms are not ours to assume.** A TCIA collection carries
   its own licence and citation requirement per collection. Re-hosting it
   inside a repository that will be public changes who is redistributing it.
   The manifest records the licence and its URL per case so the question has
   an answer without the bytes being here.
2. **PHI risk is not zero just because a set is labelled de-identified.**
   Burned-in pixel annotation is a real and common failure, which is why
   HLD E22.3 is a story. A repository that never contains DICOM cannot leak
   one. `.githooks/pre-commit` refuses a staged `.dcm` for that reason.
3. **Size.** A corpus large enough to be worth rendering is larger than a
   repository should be.

## What the manifest guarantees

Every row carries `sha256`, so a local corpus is either the corpus the
tolerance policy was written against or it is not, and `scripts/corpus_check.py`
says which. A silently different corpus is how a green suite stops meaning
anything.

`bin/ocelli.sh gate corpus` also reads the non-patient DICOM attributes that
drive coverage. It compares Modality, Transfer Syntax UID and the pixel-module
facts behind `mono16`, `colour` and `us` with each manifest row. A correct
digest with a mistyped label therefore fails rather than claiming coverage the
corpus does not have. Failure output contains only the relative corpus path and
the mismatched attribute name.

Columns: `path`, `modality`, `transfer_syntax`, `category`, `source`,
`licence`, `licence_url`, `sha256`, `url`.

`url` may be empty for a case obtained out of band. The row is still checked
for presence and digest, it simply cannot be fetched. `scripts/corpus_check.py
--fetch` downloads only the rows that carry one. Every real case here has an
empty `url`, because TCIA serves a series as an archive rather than as a stable
per-file address.

### The `category` column is a token list, and it is read by a check

Comma separated. Two of the tokens are structural and
`scripts/corpus_check.py --coverage` fails a row that is missing either.

| Token | Meaning |
|-------|---------|
| `synthetic` or `real` | Which layer the case belongs to. Exactly one |
| `mono16` | HLD 25.1 tolerance class one, monochrome 16-bit |
| `colour`, `us` | HLD 25.1 tolerance class two, colour and ultrasound |
| anything else | What trap the case exists for, free text |

A row that declares no tolerance class is a hole rather than a case, because
the tolerance it would be compared under is undecidable.

## The two layers

A corpus built only from real public studies cannot be relied on to contain a
signed 12-bit-in-16 CT with `HighBit` 15, a `MONOCHROME1` with a known
gradient, a non-square `PixelSpacing`, or a deliberately non-uniform slice
spacing. A corpus built only from synthetic cases has never seen a real
vendor's padding, private blocks or odd-length values. So there are two, and
the `category` column records which.

### Layer 1, synthetic, regenerated from a committed script

```bash
export OCELLI_CORPUS_DIR=/path/to/your/corpus
python3 scripts/corpus_synth.py                  # writes synthetic/ and syntax/
python3 scripts/corpus_synth.py --write-manifest # refresh those rows only
python3 scripts/corpus_check.py                  # digests must still match
```

Prerequisites, all native:

| Prerequisite | Why it is needed |
|--------------|------------------|
| `pydicom` | writes every file, and encodes RLE, JPEG-LS and JPEG 2000 |
| `numpy` | the pixel arrays |
| `pylibjpeg`, `pylibjpeg-libjpeg`, `pylibjpeg-rle` | decoder plugins, so the conformance tests can read back what was written |
| `pylibjpeg-openjpeg` | JPEG 2000 and HTJ2K |
| `pyjpegls` | JPEG-LS |
| **DCMTK** (`dcmcjpeg` on `PATH`) | the four JPEG syntaxes pydicom has no encoder for |
| **OpenJPH** (`ojph_compress` on `PATH`) | the HTJ2K codestream, which the generator then encapsulates |

**The versions matter**, and they are not written down here. They are
`BUILT_WITH` in `scripts/corpus_synth.py`, beside the code that depends on
them, covering the ones that can move a digest rather than every package above:

```bash
python3 scripts/corpus_synth.py --tool-versions   # what you have, against those
```

That command asks each tool rather than trusting anything written down, and it
is the way to tell a toolchain bump from a corrupted corpus when a digest
moves. OpenJPH has no version flag, so it is asked the only way that matters,
by reading the comment marker out of a codestream it just wrote. A tool that is
not installed is reported as `absent` in the table rather than as a traceback,
because this is the command you run when something is already wrong.

**CI pins everything here except DCMTK**, which comes from the Ubuntu
distribution and is 3.6.7 there. That costs nothing, because the
`corpus-tooling` job never compares a digest against this manifest.
**Regenerating the manifest is a local operation**, on the machine that holds
the corpus.

`scripts/corpus_synth.py` is committed and the files it writes are not, which
is the right way round twice over. `.githooks/pre-commit` refuses the files
anyway, and a script says what each case is for where a binary does not.

**The generator is byte-deterministic and that is a hard requirement.** UIDs
are derived by hash from the case name inside the `2.25.` arc, dates are fixed,
and every file is written by one writer with a fixed implementation identity
even when an external tool produced the codestream inside it. A regenerated
corpus therefore has identical digests and the manifest keeps meaning
something.

One thing determinism cannot cover: a codestream carries whatever its encoder
produced. Which encoder owns which case is `EXTERNAL_ENCODERS` in
`scripts/corpus_synth.py`, and `--tool-versions` prints it from there when a
version has moved. **The encoders that leave no version in the file are the
ones to be careful about**, because for those nothing but that table says a
bump happened.

That is the manifest working rather than failing. It is telling you the thing
the tolerance policy was measured against moved, which is exactly the signal
`.claude/WORKFLOW.md` asks you to check before suspecting the code. Run
`--tool-versions` first: if a version moved, the corpus did not rot, the
toolchain changed, and the two want telling apart before anyone edits a digest.

### Layer 2, real, from The Cancer Imaging Archive

One small series per class, which is what the oracle needs to have seen a real
vendor file in each tolerance class. Each collection publishes its own licence
and a DOI, and both are recorded per row. The assessment against the three
questions in `docs/SOURCE-POLICY.md` is written up there under "Extensions to
the table".

| Local path | Collection | Modality | Files | Pixels | Licence |
|------------|-----------|----------|-------|--------|---------|
| `real/ct_cmb_mml/` | CMB-MML | CT | 27 | 16-bit signed MONOCHROME2 | CC BY 4.0 |
| `real/mr_eay131/` | EAY131 | MR | 15 | 12-in-16 MONOCHROME2, Implicit VR | CC BY 4.0 |
| `real/dx_varepop/` | VAREPOP-APOLLO | DX | 1 | 12-in-16 MONOCHROME2 | CC BY 4.0 |
| `real/us_cmb_crc/` | CMB-CRC | US | 1 | 8-bit MONOCHROME2, **no chroma** | CC BY 4.0 |

**The real layer has no chroma in it, and that is a gap worth knowing about.**
HLD section 25.1's second tolerance class is "colour and ultrasound", and the
reason it gives for the class is that "chroma subsampling and YBR conversion
legitimately differ". The ultrasound case above satisfies the class as the
policy words it and exercises neither of those things, because it is greyscale.
Every byte of chroma in this corpus is generated by this repository.

So the row carries the tokens `greyscale-8bit` and `chroma-untested`, and
`corpus_check.py --coverage` prints a note whenever the real layer has a
class-two row and none of them carries a `colour` token. (With no real
class-two row at all, coverage fails outright for a louder reason.) Adding a
real colour case removes the note, and `--coverage` then also refuses the now
contradictory `chroma-untested` token on that row, so the gap cannot be
recorded as open after it has been closed. The check does not fail on it,
because failing would mean disagreeing with the policy it implements. A real
colour or Doppler ultrasound series is the fuller answer and is a later
addition, not something F-009 owed. Separately, HLD 25.1 states no tolerance
for 8-bit monochrome at all, and this file is absorbed into class two by
modality.

To reproduce the acquisition, series by series:

```bash
export OCELLI_CORPUS_DIR=/path/to/your/corpus
API=https://services.cancerimagingarchive.net/nbia-api/services/v1

# The four series, by SeriesInstanceUID.
CT=1.3.6.1.4.1.14519.5.2.1.108975852603347259500108190173730050021
MR=1.3.6.1.4.1.14519.5.2.1.1620.1226.229417808443818737599259533657
DX=1.3.6.1.4.1.14519.5.2.1.111496736574540772816177955707250560822
US=1.3.6.1.4.1.14519.5.2.1.1.56314755871495081827678310314743171188

for pair in ct_cmb_mml:$CT mr_eay131:$MR dx_varepop:$DX us_cmb_crc:$US; do
  name=${pair%%:*}; uid=${pair#*:}
  mkdir -p "$OCELLI_CORPUS_DIR/real/$name"
  curl -s -o "/tmp/$name.zip" "$API/getImage?SeriesInstanceUID=$uid"
  ( cd "$OCELLI_CORPUS_DIR/real/$name" && unzip -q "/tmp/$name.zip" )
done
```

Each archive contains the instances plus a `LICENSE` file naming the
collection's terms. Keep it. It is the evidence behind the `licence` column,
and it is not itself a corpus case, so it gets no manifest row.

The per-series licence can also be read back from the API, which is where the
manifest's values came from:

```bash
curl -s "$API/getSeries?Collection=CMB-MML" | python3 -m json.tool | \
  grep -E 'License|SeriesInstanceUID'
```

Then add each file with the digest computed for you:

```bash
python3 scripts/corpus_check.py --add "$OCELLI_CORPUS_DIR/real/ct_cmb_mml/00000001.dcm" \
  --modality CT --transfer-syntax 1.2.840.10008.1.2.1 \
  --category "real, mono16, series, burned-in-unchecked" \
  --source "TCIA CMB-MML, https://doi.org/10.7937/SZKB-SW39" \
  --licence "CC BY 4.0" --licence-url "https://creativecommons.org/licenses/by/4.0/" \
  --url ""
```

**Read the transfer syntax out of each file rather than assuming it.** The MR
series above is Implicit VR Little Endian and the other three are Explicit VR
Little Endian, which is not something the collection page tells you.

### `burned-in-unchecked`, and why every real row carries it

A collection labelled de-identified can still carry burned-in pixel
annotation. HLD story E22.3 exists to detect that and it is not built, so every
real row is marked and the gap is visible rather than assumed away. Synthetic
cases have no patient identity to remove and carry no such marker.

## Coverage, and how it is checked

```bash
python3 scripts/corpus_check.py --coverage   # manifest only, no data needed
bin/ocelli.sh gate corpus                    # coverage, then the digests
```

`--coverage` fails naming what is absent when any of these is untrue:

- every transfer syntax the codec registry will claim has at least one row,
  so that Appendix A gates A1 for HTJ2K and A2 for JPEG-LS have something to
  be answered against
- both tolerance classes of HLD section 25.1 are represented
- at least one row is not synthetic, and the real rows cover both classes
- every row declares a layer and a tolerance class

It reads the manifest and nothing else, deliberately. Deviation D-04 leaves CI
without a GPU and without the corpus, and coverage is the part of this story
that survives that, so CI runs it: the `guards` job of
`.github/workflows/ci.yml`. The `corpus` gate itself stays out of
`bin/ocelli.sh gate --floor` because its other half verifies digests against a
corpus CI does not have.

## Running the checks on the tooling itself

```bash
bin/ocelli.sh gate corpus-tests      # what /verify runs, see below for CI
```

That is the gate. It is in the CI floor, because it needs no corpus and no GPU:
the generator writes into a temporary directory of its own.

Under the hood it is `python3 scripts/corpus_tests.py`, and there are two
reasons it is a script rather than a line of shell.

**It fails on a skipped test rather than on the exit status.** Run the suites
under an interpreter with no pydicom and the generator suite reports a single
skip, the process exits 0, and the hand-computed PS3.3 fixture did not run. A
skip is not a pass anywhere else in this project and it is not one here.

**It resolves the interpreter, because that is a property of the machine.** The
generator needs pydicom, numpy and the codec plugins, and a checkout cannot know
where that lives. First match wins, and each candidate is accepted only if it
can actually import what is needed:

1. `$OCELLI_PYTHON`
2. `.ocelli-python-path`, per clone and gitignored
3. the interpreter running the script, which is the whole answer in CI
4. `ocelli-tools/venv/bin/python` beside the checkout or beside its parent

```bash
python3 scripts/corpus_tests.py --which          # what it resolved, and from where
python3 scripts/corpus_tests.py --set /path/to/python   # record it for this clone
```

The first two are authoritative: if one is set and cannot import pydicom, no
fallback is tried **for the generator suite**, because running a different
interpreter from the one asked for and reporting success is its own quiet
failure. The coverage suite needs nothing but the standard library, so it runs
under `sys.executable` regardless, and the report then omits the `interpreter:`
line rather than claiming one that was rejected.

When nothing resolves, or DCMTK or OpenJPH is absent, the generator half exits
3, which the gate runner counts and names as SKIPPED and never as a pass.

**A caller that installed the prerequisites should not accept that skip**, and
CI is such a caller:

```bash
python3 scripts/corpus_tests.py --require-prerequisites   # a skip is a failure
```

`bin/ocelli.sh` returns 0 for a skipped gate, which is right for `docs` and
`wasm` whose skips are permanent, and wrong for the `corpus-tooling` CI job
whose earlier steps exist precisely to remove every reason to skip. So that job
calls the runner directly with this flag. Without it, an OpenJPH build that
installed outside `PATH` would give the job a green tick having run only the
coverage suite.

## What this corpus still does not have

Named here rather than left to be discovered, because a gap someone knows about
is worth more than a row nobody can evidence.

- **No real chroma.** See the real-layer table above.
- **The `j2k_*` and `jpegls_*` cases are encoded and decoded by the same
  library**, OpenJPEG and pyjpegls respectively, so the conformance check on
  those is weaker than on the rest. The `jpeg_*` and `htj2k_*` cases are
  decoded by a different library from the one that encoded them.
- **No encapsulation edge cases.** Every compressed case is one frame in one
  fragment with a populated Basic Offset Table. Multi-fragment frames, a
  multi-frame encapsulated instance and an empty Basic Offset Table are three
  named traps in PS3.5 A.4 that nothing here exercises. They belong with the
  codec story, and every field bug becomes a permanent fixture under E2.6.

## Adding a case

Every field bug becomes a permanent fixture (HLD section 11, story E2.6). To
add one:

1. Put the file under `$OCELLI_CORPUS_DIR`.
2. `python3 scripts/corpus_check.py --add <file> --modality CT
   --transfer-syntax <uid> --category ...` appends the row with a computed
   digest. Three of those are load-bearing and all three default to empty:
   `--transfer-syntax` is what condition 4 is counted from, and `--category`
   needs a layer token and a tolerance class token, see the table above.
   `--coverage` names the row by path if any is missing.

   **Read the transfer syntax out of the file, do not assume it.**

   ```bash
   dcmdump +P "0002,0010" <file>      # or ask pydicom, either way ask
   ```
3. Run `bin/ocelli.sh gate corpus`. This checks the digest and compares the
   coverage labels with the file metadata using the configured DICOM Python
   interpreter.
4. Commit the manifest row, never the file.

A synthetic case is added by teaching `scripts/corpus_synth.py` to write it and
re-running `--write-manifest`, not by hand. That command replaces only the rows
under `synthetic/` and `syntax/` and leaves every other row alone.

The tolerance the case is compared under comes from the policy in
`docs/hld/22-testing-and-tolerance.md` section 25.1. It is not set per case.
A tolerance change is a pull request with a rationale, reviewed like code.
