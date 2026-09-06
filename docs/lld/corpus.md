# The golden corpus

**F-IDs that contributed:** F-009, F-X006, F-X007, F-X012, F-X013
**Last updated:** 2026-09-06

The corpus is the input every later correctness claim is measured on. It lives
outside git under ignored `corpus/data` behind `corpus/manifest.tsv`, which is
deviation **D-05**. `corpus/README.md` is the operator-facing guide. This file
is the design behind it.

**Read [oracle.md](oracle.md) beside this one.** The corpus is the input and
the oracle is what renders it, so the two are one instrument. That file records
which rows cornerstone3D 5.8.2 cannot render and why, which is a fact about the
reference rather than about the corpus.

**Since F-X007 the sixty-two `series` rows are exercised as geometry as well as
pixels.** The oracle attempts all four series directories and
assembles three of them into volumes, rendering three orthogonal reformats of
each. The fourth, `real/ct_cmb_mml`, is refused as declared, because its 27
instances resolve to only 9 distinct positions on the slice normal and a series
that cannot be one volume is named rather than averaged. A row's projected
`ImagePositionPatient` therefore decides something rather than only travelling
in a sidecar. Where a row sits, and whether its directory is one spatial volume
at all, is now a checked claim. See "Stack and volume, and what each covers" in
`oracle.md`.

## Two layers, because neither alone is enough

The `category` column records which layer a case belongs to.

**Synthetic**, written by `scripts/corpus_synth.py` into `synthetic/` and
`syntax/`. A corpus built only from real studies cannot be relied on to contain
a signed 12-bit-in-16 CT with `HighBit` 15, a `MONOCHROME1` with a known
gradient, a non-square `PixelSpacing`, or a deliberately non-uniform slice
spacing. Each synthetic case exists to make one trap detectable and its pixel
values are hand-predictable. The trap each one is for is written on the
function that generates it, and is not repeated here, because a second copy of
that list would be free to drift from the cases themselves.

**Real**, four TCIA series under `real/`. A corpus built only from generated
cases has never seen a vendor's padding, private blocks or odd-length values.
One of the four is Implicit VR Little Endian, which no synthetic case would
have produced by accident.

F-X012 adds `synthetic/ct_sigmoid_width_half.dcm`, a 12 by 20 CT whose stored
values 151 through 170 become modality values 37.75 through 42.5 under slope
0.25 and intercept 0. Its file window is centre 40, width 0.5 and function
SIGMOID. The width is deliberately below LINEAR's minimum while remaining
legal under PS3.3 C.11.2.1.3.1. The fixture transcribes that section's formula
independently and pins display values 4.586483540333347, 30.396745115639977,
127.5, 224.60325488436 and 250.41351645966665 at modality values 39.5 through
40.5. No renderer supplies those answers.

## The `category` column is a token list, and a check reads it

Comma separated. Two token classes are structural and
`scripts/corpus_check.py --coverage` fails a row missing either.

| Token | Meaning |
|-------|---------|
| `synthetic`, `real` | the layer. Exactly one |
| `mono16` | HLD 25.1 tolerance class one |
| `colour`, `us` | HLD 25.1 tolerance class two |
| anything else | which trap the case exists for |

`--coverage` reads the manifest and nothing else, so it answers under deviation
D-04 where CI has neither a GPU nor the corpus. The `guards` job of
`.github/workflows/ci.yml` runs it. It fails, naming what is absent, when a
transfer syntax the codec registry claims has no row, when a row declares no
layer or no tolerance class, when a row's `transfer_syntax` is blank, when a
row carries a claim and its own contradiction (`colour` beside
`chroma-untested`), when either tolerance class is unrepresented in the corpus
or in the real layer alone, or when every row is synthetic.

`bin/ocelli.sh gate corpus` runs `--coverage`, digest verification and a
corpus-present metadata audit, chained so that any failure remains a failure.
The audit uses pydicom through the configured tooling interpreter and reads
only Modality, Transfer Syntax UID, Samples per Pixel, Photometric
Interpretation and Bits Allocated. Those are the non-patient attributes that
decide the manifest's modality and tolerance-class claims. It reports only a
relative corpus path and the mismatched attribute name.

## Byte-determinism is a hard requirement

The manifest records a sha256 per case, so a generator that stamps the clock or
mints a fresh UID produces a different digest on every machine and the manifest
stops meaning anything. UIDs are derived by hash from the case name inside the
`2.25.` UUID-derived arc, dates are fixed, and every file is written by one
writer with a fixed implementation identity even when an external tool produced
the codestream inside it.

**The determinism test spawns two subprocesses, and that is the point.** Both
generations inside one interpreter would prove much less than it looks like:
a module-level constant taken from the clock, the process id or a fresh UUID is
evaluated once at import and then agrees with itself for the rest of the run.
Two processes at two wall-clock moments is the cheapest thing that catches that
class. A separate test asserts no case carries a clock reading and that every
instance UID sits in the `2.25.` arc.

## Which encoders leave a version, and which do not

`EXTERNAL_ENCODERS` in `scripts/corpus_synth.py` maps transfer syntax to
producer. It is keyed by syntax rather than by filename so the case names live
in `SYNTAX_CASES` and nowhere else, and every cell of it is asserted against
the written bytes.

DCMTK and pyjpegls leave no version anywhere in what they produce. OpenJPEG and
OpenJPH each write one into a COM marker. **The ones that leave nothing are the
ones to watch**, because for those a bump moves a digest silently, and
`scripts/corpus_synth.py --tool-versions` against `BUILT_WITH` is the only
thing that will say so. DCMTK does write a `DerivationDescription` on every
case it encodes and nothing else here does, which is what tells its cases from
pyjpegls's, and a test asserts that in both directions.

RLE is the one compressed syntax no external encoder touches. pydicom
implements it, and both plugins it offers produce byte-identical output.

## Two corrections F-X007 measured

**`scripts/corpus_synth.py`'s comment on `NONUNIFORM_SLICE` is wrong about the
error's shape.** It says a volume builder that averages instead of refusing
"produces a plausible reformat that is wrong by a constant factor". On this
construction it is not a constant factor. Slice 7 is displaced by half a gap, so
the two gaps either side become 3.75 and 1.25, and the displacement cancels out
of the endpoint difference: the mean gap is 22.5 / 9, which is 2.5, and so is
the median, and so is `SliceThickness`, exactly as in the uniform series. The
error an averaging builder makes is **one slice placed 1.25 mm out of position**,
which is half a slice gap, and every other slice is exactly right. That is a
harder defect to see than a scale error, not an easier one, so the case is
better than its comment claims.

The generator's comment is not edited by this story. Changing a comment in
`scripts/corpus_synth.py` does not change a digest, but it invites a
regeneration nobody asked for, and the manifest is what ties every reference
frame to a corpus.

**`real/ct_cmb_mml` is not one spatial volume, and the corpus did not say so.**
Its twenty-seven instances occupy only NINE distinct positions on the slice
normal, four instances deep at six of those positions and one at the other
three, across three values of `AcquisitionNumber` (0020,0012). It is a
multi-acquisition series, and PS3.3 C.7.6.2.1.1 defines no spacing between two
instances at the same position, because their projected difference is zero and
zero is not a gap.

That is a fact about a real TCIA series rather than a defect, and it is worth
having: it is what real multi-acquisition data looks like, and no synthetic case
would have produced it by accident. cornerstone3D 5.8.2 assembles a volume from
it without complaint, spreading nine real positions over twenty-six imaginary
ones and relabelling each instance into the frame slot of its ordinal position.
The oracle refuses it by name instead, and `tools/oracle/volume-truth.json`
declares that refusal with its reason, so the run stays green and the reason is
in the output rather than in somebody's memory.

`real/mr_eay131` is the counter-case and is a proper spatial volume: fifteen
instances at fifteen distinct positions, one acquisition. Its gaps are not
uniform, which is ordinary for real data and is measured rather than judged.

## What the corpus does not have, and it is recorded rather than assumed

- **No real chroma.** The real class-two case is an 8-bit `MONOCHROME2`
  ultrasound. HLD 25.1 gives the reason for that class as chroma subsampling
  and YBR conversion, and a greyscale ultrasound exercises neither, so every
  byte of chroma in the corpus is generated by this repository. The row carries
  `chroma-untested`, `--coverage` prints a note while no real row carries a
  `colour` token, and the check does not fail on it, because failing would mean
  disagreeing with the policy it implements.
- **HLD 25.1 states no tolerance for 8-bit monochrome at all.** That ultrasound
  case is absorbed into class two by modality.
- **The `j2k_*` and `jpegls_*` cases are encoded and decoded by the same
  library**, so the conformance check on them is weaker than on the rest.
  **F-X006 narrowed this for the two `jpegls_*` rows and for neither `j2k_*`
  one.** `pure_jpegls` 2.0.0 is a standard implementation rather than a CharLS
  port, and it decoded both: byte-identical to the uncompressed reference `R`
  for `.80`, which is encoder-independent because `R` is
  `scripts/corpus_synth.py`'s ramp rather than any codec's output, and within
  ISO/IEC 14495-1's declared `NEAR` of 3 for `.81`. **`dcmdjpls` agreeing is
  not independent evidence**, because DCMTK and the `pyjpegls` that encoded
  these rows both wrap CharLS, and `docs/spikes/A2-jpeg-ls.md` says so beside
  its own digests. The `j2k_*` rows gained nothing from that spike:
  `openjp2` 0.6.1 is a C2Rust port of the OpenJPEG that encoded them, so its
  native decode is the same library on both sides, and `j2k_lossy` was compared
  against no other party at all. `docs/spikes/A1-htj2k-openjp2.md` carries
  those digests. F-X013 adds a different answer for the three `htj2k_*` rows:
  `openjph-core` 0.1.0 reproduces the synthetic ramp exactly for `.201` and
  `.202` on native and wasm. That ramp is the independent anchor. For `.203`,
  the crate and `ojph_expand` share OpenJPH lineage, so their 41 one-level
  differences are a measured divergence and not independent confirmation.
  `docs/spikes/A1-htj2k-route.md` carries the exact digests and limitation.
- **No encapsulation edge cases.** Every compressed case is one frame in one
  fragment with a populated Basic Offset Table, so multi-fragment frames, a
  multi-frame encapsulated instance and an empty Basic Offset Table are
  untested. They belong with the codec story under E2.6.
- **No real series that is one clean spatial volume at CT.** `real/mr_eay131`
  is one at MR, and `real/ct_cmb_mml` is multi-acquisition, so the only CT
  volume the oracle assembles is synthetic. A real CT series at one acquisition
  would be worth adding, and until it exists the CT volume reference F-011
  compares against is a generated ramp.

## The pydicom test files, refused

The pydicom project ships test files covering most of the registry syntaxes and
they are the obvious shortcut. Its own `test_files/README.txt` says of them,
verbatim, "I believe there is no restriction on using any of these files in
this manner", and traces individual files to several upstream sources with
differing terms. **A belief is not a grant**, and the manifest requires a
per-case `licence_url` someone could act on, which cannot be written from that
sentence. The assessment is recorded in `docs/SOURCE-POLICY.md` under
"Extensions to the table" so the next person to look finds the reason rather
than a silent absence.
