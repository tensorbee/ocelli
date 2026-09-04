# The golden corpus

**F-IDs that contributed:** F-009
**Last updated:** 2026-09-04

The corpus is the input every later correctness claim is measured on. It lives
outside git at `$OCELLI_CORPUS_DIR` behind `corpus/manifest.tsv`, which is
deviation **D-05**. `corpus/README.md` is the operator-facing guide. This file
is the design behind it.

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
- **No encapsulation edge cases.** Every compressed case is one frame in one
  fragment with a populated Basic Offset Table, so multi-fragment frames, a
  multi-frame encapsulated instance and an empty Basic Offset Table are
  untested. They belong with the codec story under E2.6.

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
