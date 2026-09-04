# Current sprint, S01

**Milestone**: M1, foundations and the differential oracle.
**Branch**: `sprint/s01`
**Opened**: bootstrap
**Goal**: The workspace is real and the corpus exists. Nothing is ported.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-001 | E1.1 | Cargo workspace, crate skeleton, lint/CI baseline | Build | 2w | done |
| F-009 | E2.1 | Golden corpus ingest and de-identified fixture store | Test | 3w | pending |

## What this sprint is

**Two stories, and the second one is the important one.** F-001 is largely
present already: the bootstrap commit created the workspace, the thirteen
crates, the lint baseline and the gate runner. What F-001 still owes is the
`ocelli-core` types that everything downstream depends on, entries 1 and 2 of
`docs/hld/25-first-ten-files.md`.

F-009 is the one that gates the programme. Until a corpus exists there is
nothing for the oracle to render, and until the oracle renders there is no
mechanism that makes generated Rust safe to merge (HLD decision D7).

## The order is not negotiable

`docs/hld/25-first-ten-files.md` gives the first ten files in order and says
what the first two weeks are for: **diff one windowed 2D image against
cornerstone3D.** Everything in M1 and M2 serves that.

Entry 4 in that list is `tools/oracle/`, with the note "nothing else should
start before this works". That is a real constraint on sprint order, not a
preference. A tempting alternative is to start the DICOM parser in M2 while the
corpus is still being assembled, because parsing is well understood and feels
like progress. Doing that means the first thing the oracle checks is a body of
code written without it, which is the exact situation D7 exists to prevent.

## What "done" means for F-009

Not "a directory of DICOM files exists". The corpus is not in git
(`corpus/README.md`), so done means:

1. `corpus/manifest.tsv` carries a row per case with a real sha256, and a
   licence and licence URL that someone could act on.
2. `python3 scripts/corpus_check.py` passes against `$OCELLI_CORPUS_DIR`.
3. The modality spread covers what the tolerance policy distinguishes, which
   is at minimum monochrome 16-bit (CT, MR, CR or DR) and colour or ultrasound,
   because HLD section 25.1 sets different tolerances for those two classes and
   an untested class has an untested tolerance.
4. At least one case per transfer syntax the codec registry will claim, so
   that the two open gates in Appendix A, HTJ2K and JPEG-LS, have something to
   be answered against.

## Standing expectations

**Every guard is seen red before it is claimed.** Remove or revert the thing a
guard protects, one at a time, and watch it fail. A guard that has never been
observed failing is a guard nobody has tested, and this repository now has
seventeen of them written in one sitting by one author. Treat that as a
liability until each has been seen red once.

**Read the normative source before implementing.** `docs/hld/` is cut from the
authored document and is prescriptive in Part II. Where it gives a formula, a
layout or a signature, that is the implementation, and a deviation is raised in
`docs/hld/DEVIATIONS.md` rather than improvised.

**No pixel arithmetic lands without a hand-computed fixture citing its DICOM
section** (HLD 27.2 R3). The four rows of the section 18.3 table are the
worked example, and they must be in the test suite before the shader is
written.
