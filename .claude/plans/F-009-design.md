# F-009, Golden corpus ingest and de-identified fixture store

**Status**: approved
**Epic ref**: E2.1
**Sprint**: S01
**Estimate**: 3w

## Normative source, transcribed

### `docs/hld/08-validation-architecture.md`, section 11, verbatim

> Cornerstone3D is a correct reference implementation that can render any
> series you own. The harness pushes the same study through both stacks and
> compares frames within a written per-modality tolerance, with metadata
> diffed alongside pixels because a wrong rescale slope can still produce a
> plausible image.
>
> Every pull request renders the corpus in CI. Every field bug becomes a
> permanent fixture. In production, shadow mode renders both libraries and
> alerts on divergence - the oracle running against real clinical traffic, and
> the same corpus a regulatory submission would want to see.

### `docs/hld/22-testing-and-tolerance.md`, section 25.1, verbatim

> Write it down once and hold it. Tuning tolerance per failure is how a suite
> stops meaning anything.
>
> - **Monochrome 16-bit (CT, MR, CR, DR):** maximum absolute difference <= 1
>   LSB on at least 99.9% of pixels, zero pixels differing by more than 2.
> - **Colour and ultrasound:** perceptual difference below a stated threshold,
>   because chroma subsampling and YBR conversion legitimately differ.
> - **Geometry:** world coordinates within 1e-6 mm, canvas coordinates within
>   a quarter pixel.
> - A tolerance change is a pull request with a rationale, reviewed like code.

**The consequence this story is built on**: the policy distinguishes two pixel
classes, and an untested class has an untested tolerance. So the corpus is not
done until both classes are present.

### `docs/sprints/CURRENT_SPRINT.md`, the four conditions for done, verbatim

> 1. `corpus/manifest.tsv` carries a row per case with a real sha256, and a
>    licence and licence URL that someone could act on.
> 2. `python3 scripts/corpus_check.py` passes against `$OCELLI_CORPUS_DIR`.
> 3. The modality spread covers what the tolerance policy distinguishes, which
>    is at minimum monochrome 16-bit (CT, MR, CR or DR) and colour or
>    ultrasound, because HLD section 25.1 sets different tolerances for those
>    two classes and an untested class has an untested tolerance.
> 4. At least one case per transfer syntax the codec registry will claim, so
>    that the two open gates in Appendix A, HTJ2K and JPEG-LS, have something
>    to be answered against.

### `docs/hld/DEVIATIONS.md`, D-05, verbatim

> The corpus lives outside git at `$OCELLI_CORPUS_DIR`, with a committed
> manifest of per-case checksums and metadata. Operator constraint. A
> TCIA-derived corpus is large and its redistribution terms are not ours to
> assume. The manifest makes the corpus verifiable without being present.

### `docs/SOURCE-POLICY.md`, the rule for a source not yet listed, verbatim

> **No licence is not the same as permissive.** A repository with no LICENSE
> file, and no `license` field in its metadata, is **all rights reserved** by
> default under the Berne Convention.
>
> Before taking anything from a source not in the table above, check three
> things and record the answer:
>
> 1. Is there a LICENSE, LICENCE, COPYING or NOTICE file at the repository
>    root?
> 2. Does the hosting platform's metadata report a licence?
> 3. Does the licence permit the specific use, which for source is usually
>    **derivative works**, not merely use?
>
> If 1 and 2 are both absent, the answer is no.

### The transfer syntaxes the codec registry will claim

From the `dicom-expert` skill's table, which is `docs/hld/18-codec-registry.md`
plus PS3.5, and from the `Ocelli` column of that table:

| UID | Name | Registry |
|-----|------|----------|
| `1.2.840.10008.1.2` | Implicit VR Little Endian | native |
| `1.2.840.10008.1.2.1` | Explicit VR Little Endian | native |
| `1.2.840.10008.1.2.1.99` | Deflated Explicit VR LE | deflate |
| `1.2.840.10008.1.2.2` | Explicit VR Big Endian, retired | native, byte-swapped |
| `1.2.840.10008.1.2.5` | RLE Lossless | rle |
| `1.2.840.10008.1.2.4.50` | JPEG Baseline, process 1, 8-bit | jpeg |
| `1.2.840.10008.1.2.4.51` | JPEG Extended, process 2 and 4, 12-bit | jpeg |
| `1.2.840.10008.1.2.4.57` | JPEG Lossless, process 14 | jpeg |
| `1.2.840.10008.1.2.4.70` | JPEG Lossless, process 14 SV1 | jpeg |
| `1.2.840.10008.1.2.4.80` | JPEG-LS Lossless | open gate A2 |
| `1.2.840.10008.1.2.4.81` | JPEG-LS Near-Lossless | open gate A2 |
| `1.2.840.10008.1.2.4.90` | JPEG 2000 Lossless Only | openjp2 |
| `1.2.840.10008.1.2.4.91` | JPEG 2000 | openjp2 |
| `1.2.840.10008.1.2.4.201` | HTJ2K Lossless Only | open gate A1 |
| `1.2.840.10008.1.2.4.202` | HTJ2K Lossless Only, RPCL | open gate A1 |
| `1.2.840.10008.1.2.4.203` | HTJ2K | open gate A1 |

Fifteen syntaxes. Condition 4 is therefore fifteen rows at minimum, not a
gesture at "the common ones".

## What the specification does not cover

The HLD says the corpus exists and what it is for. It does not say where the
bytes come from, and this is where the whole story lives.

1. **Two layers, not one.** A corpus built only from real public studies cannot
   be relied on to contain a signed 12-bit-in-16 CT with `HighBit` 15, a
   `MONOCHROME1` with a known gradient, a non-square `PixelSpacing`, or a
   deliberately non-uniform slice spacing. A corpus built only from synthetic
   cases has never seen a real vendor's padding, private blocks or odd-length
   values. **Decision**: both layers, and the manifest's `category` column
   records which.
2. **Synthetic cases must be byte-deterministic.** The manifest is a sha256 per
   case, so a generator that stamps today's date or a fresh UID produces a
   different digest on every machine and the manifest stops meaning anything.
   **Decision**: `scripts/corpus_synth.py` uses fixed UIDs from a private
   `2.25.` namespace, a fixed date, and writes with an explicit transfer
   syntax, so a regenerated corpus has identical digests. This is asserted by
   a test that generates twice and compares.
3. **Coverage is a claim unless it is checked.** Conditions 3 and 4 are
   properties of the manifest, and a plan that reads them and says "yes" is
   the thing this project distrusts. **Decision**: `scripts/corpus_check.py`
   gains a `--coverage` mode that reads the manifest and fails naming what is
   missing, and `bin/ocelli.sh gate corpus` runs it. Coverage is then checkable
   with no corpus present, which matters because it is the part CI can still
   see under D-04.
4. **Licence per case, not per corpus.** `corpus_check.py` already refuses an
   empty `licence` or `licence_url`. What it cannot check is whether the
   recorded licence is true. **Decision**: every row's licence comes from a
   source whose terms were assessed against the three SOURCE-POLICY questions,
   and the assessment is recorded as a table row in `docs/SOURCE-POLICY.md`
   under "Extensions to the table", which that file explicitly provides for.
5. **A widely used source that this policy refuses.** The pydicom project ships
   176 test files covering most of the fifteen syntaxes, and it would be the
   obvious shortcut. Its own `test_files/README.txt` says of them, verbatim,
   "I believe there is no restriction on using any of these files in this
   manner", and traces individual files to NEMA WG04, `dclunie.com`,
   `barre.nom.fr` and GDCM. A belief is not a grant, and a per-case
   `licence_url` "someone could act on" cannot be written from it. **Decision**:
   not used as a corpus source. Recorded here because the next person to look
   will find it first and deserves the reason rather than a silent absence.
6. **De-identification.** The story title says "de-identified fixture store".
   Synthetic cases have no patient identity to remove. Real cases come from a
   collection that is already de-identified under its own published process,
   and the corpus is outside git, so the repository's exposure is zero either
   way. What remains is burned-in pixel annotation, which HLD story E22.3
   exists for and which this story cannot solve. **Decision**: the manifest's
   `category` column carries a `burned-in-unchecked` marker for any real case
   until E22.3 can check it, so the gap is visible rather than assumed away.

## Approach

### Layer 1, synthetic, `scripts/corpus_synth.py`

A committed generator, per the `dicom-tooling` skill section 5: the script is
better than a committed binary because it says what each case is for, and the
pre-commit hook refuses the binary anyway.

Each case is small, has hand-predictable pixel values, and exists to make one
specific trap detectable. The base cases, all written under
`$OCELLI_CORPUS_DIR/synthetic/`:

| Case | What it makes detectable |
|------|--------------------------|
| `ct_signed_12in16_right.dcm` | `BitsStored` 12, `HighBit` 11, signed. The mask and sign-extend path |
| `ct_signed_12in16_left.dcm` | `BitsStored` 12, `HighBit` 15. Left-aligned data, which real scanners produce |
| `ct_unsigned_16.dcm` | The plain 16-bit unsigned baseline |
| `cr_monochrome1.dcm` | `MONOCHROME1` with a known gradient. The inversion trap |
| `mr_nonsquare_spacing.dcm` | `PixelSpacing` `[0.5, 0.25]`. The transposed spacing index |
| `ct_series_uniform/` | Ten slices, uniform spacing, oblique orientation. The volume builder's happy path |
| `ct_series_nonuniform/` | Ten slices with one gap off the median. The refusal path |
| `sc_rgb_interleaved.dcm` | `RGB`, `PlanarConfiguration` 0 |
| `sc_rgb_planar.dcm` | `RGB`, `PlanarConfiguration` 1 |
| `us_ybr_full_422.dcm` | `YBR_FULL_422`, the chroma path and the colour tolerance class |
| `ct_multiframe_perframe.dcm` | Enhanced CT, rescale and window differing per frame. The per-frame-before-shared rule |

Every one carries non-square `PixelSpacing` unless the case is specifically the
square control, because a square-pixel fixture cannot catch a transposition.

### Layer 1b, transfer-syntax coverage by transcoding the synthetic base

The same pixel content is re-encoded into each syntax, so a decode can be
checked against the uncompressed original rather than against a picture. What
each syntax needs, measured on this machine rather than assumed:

| Syntax | Tool | Available now |
|--------|------|---------------|
| Implicit VR LE, Explicit VR LE, Deflated | pydicom | yes |
| Explicit VR Big Endian | pydicom, written by hand if pydicom 3 refuses | to confirm |
| RLE Lossless | pydicom `RLELosslessEncoder` | yes |
| JPEG 2000 Lossless and lossy | pydicom with `pylibjpeg-openjpeg` | yes |
| JPEG-LS Lossless and Near-Lossless | pydicom with `pyjpegls` | yes, installed |
| JPEG Baseline, Extended, Lossless 57 and 70 | DCMTK `dcmcjpeg` | installed for this story |
| HTJ2K 201, 202, 203 | OpenJPH `ojph_compress`, encapsulated by hand | installed for this story |

All fifteen are reachable. The last two rows were the open question below and
the answer was to install both formulae, so this story owes every syntax the
registry will claim rather than a subset.

**A tool used to build a case is not a tool used to check it.** DCMTK and
OpenJPH produce the codestream. Whether Ocelli decodes it correctly is decided
by the oracle against cornerstone3D, per HLD section 11, and never by
`dcmj2pnm`, which applies its own windowing and its own rounding.

### Layer 2, real clinical cases

Real cases come from The Cancer Imaging Archive, which is the source D-05
already names and the one that answers all three SOURCE-POLICY questions: each
collection publishes an explicit licence, usually CC BY 3.0 or 4.0, with a
citation requirement and a stable DOI. The per-case row records that
collection's licence and its URL, and `url` stays empty because TCIA serves
series as an archive rather than as a stable per-file URL, which
`corpus/README.md` already allows for.

Minimum real coverage, one series each: a CT, an MR, and a CR or DR, all
monochrome 16-bit, plus one colour or ultrasound case for the second tolerance
class.

### `scripts/corpus_check.py --coverage`

Reads the manifest only. Fails, naming what is absent, when any of these is not
satisfied:

- every one of the fifteen registry transfer syntaxes has at least one row
- both tolerance classes of section 25.1 are represented, monochrome 16-bit
  and colour or ultrasound
- at least one row is not synthetic, so the corpus has seen a real file, and
  the real rows cover both tolerance classes rather than only the first
- every row's `licence` and `licence_url` are non-empty, which `load` already
  enforces, restated here so the coverage report is a single answer

`bin/ocelli.sh gate corpus` runs `--coverage` first and then the digest
verification, so a manifest that has stopped covering the registry fails even
where the corpus is absent.

### `corpus/README.md` and `docs/SOURCE-POLICY.md`

`corpus/README.md` gains the acquisition procedure: where the corpus directory
lives, how to regenerate the synthetic layer, and how to obtain the real layer.
`docs/SOURCE-POLICY.md` gains an "Extensions to the table" row per real source
with the date it was decided, in the same form as the existing table, plus the
recorded refusal of the pydicom test files with its reason.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. This story produces files and a manifest, and
  no Rust runs in it.
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a. No rendering or compute path exists in this story.
- Tier B (WebGL2): n/a, same reason.
- Tier C (CPU): n/a, same reason. The corpus is the input every tier is
  measured on, and it is tier-independent by construction. That independence
  is what makes the A against C divergence bound D-07 commits to measurable at
  all.

## Tests

No pixel arithmetic is implemented in this story, so no `fixture` category
applies to Ocelli code. The arithmetic that does happen here is in the
generator, and it is checked the same way.

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | `--coverage` fails, naming the syntax, when a manifest row is removed | `scripts/tests/test_corpus_check.py` |
| `unit` | `--coverage` fails when only synthetic rows are present | `scripts/tests/test_corpus_check.py` |
| `unit` | The generator is byte-deterministic: two runs into different directories produce identical sha256 for every case | `scripts/tests/test_corpus_synth.py` |
| `fixture` | For `ct_signed_12in16_right.dcm` and `ct_signed_12in16_left.dcm`, the stored value unpacked from four hand-chosen raw words matches values computed from **PS3.3 C.7.6.3.1.4** and the `BitsStored` and `HighBit` definitions, including `0xF800` to `-2048` and `0x07FF` to `2047` | `scripts/tests/test_corpus_synth.py` |
| `conformance` | Every generated compressed case round-trips: decode it and compare against the uncompressed base case, exactly for the lossless syntaxes | `scripts/tests/test_corpus_synth.py` |

The conformance test is the one that matters. A generator that writes a
declared JPEG-LS transfer syntax around a codestream that is not JPEG-LS
produces a file that fails much later, inside the codec story, where it will
look like a decoder bug.

## Parity surface covered

None directly. The corpus is what every later parity claim is measured on, and
`docs/hld/B-parity-surface.md` carries no row a test-infrastructure story
covers.

## Deviations

None new. This story is the implementation of the existing **D-05**, the corpus
outside git behind a committed manifest, and it is read under **D-04**, which
is why the coverage check has to work with no corpus present.

## LLD impact

`docs/lld/corpus.md`, created by `/complete-feature` step 9: the two layers and
why, the synthetic case table and the trap each case exists for, the
determinism requirement, and the recorded refusal of the pydicom test files.

## Open questions

Both were asked in the S01 consolidated design round and both are answered.
Recorded here rather than deleted, because the answer is the reason the plan
looks the way it does.

1. **May Homebrew install `dcmtk` and `openjph` on this machine?**
   **Answered: install both.** So all fifteen registry transfer syntaxes are in
   scope for this story, `dcmcjpeg` supplies 50, 51, 57 and 70, and
   `ojph_compress` supplies the codestream for 201, 202 and 203. Condition 4 is
   met in full rather than partially, and both open gates, A1 for HTJ2K and A2
   for JPEG-LS, get cases to be answered against. `--coverage` therefore
   requires all fifteen and reports no permitted gap.

2. **How much real clinical data should this run download?**
   **Answered: one small series per class.** One CT, one MR, one CR or DR, and
   one colour or ultrasound, from TCIA collections whose licence and citation
   are recorded per row. This satisfies condition 3 and gives the oracle real
   vendor files, and it keeps the acquisition bounded. `--coverage`'s "at least
   one row is not synthetic" condition is therefore live rather than deferred.
   The corpus grows for the life of the project, which `allocation.json`
   already says of E2.1, so a larger multi-vendor set is a later addition and
   not a thing this story owes.
