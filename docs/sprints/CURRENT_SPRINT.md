# Current sprint, S07

**Milestone**: M2, DICOM ingest and the pixel pipeline.
**Branch**: `sprint/s07`
**Opened**: 2026-09-07
**Goal**: Turn parsed input into typed metadata and explicit source and codec
capabilities, while adding DICOMweb and NIfTI ingest without hiding a format,
network, or coordinate-system boundary.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-017 | E3.2 | Metadata model and provider registry | Rust | 4w | done |
| F-021 | E3.6 | DICOMweb client: WADO-RS, WADO-URI, QIDO-RS | Rust | 3w | done |
| F-022 | E3.7 | NIfTI volume ingest | Rust | 2w | done |
| F-023 | E4.1 | Codec dispatch layer and capability registry | Rust | 2w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together:

```bash
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S07 / {print $2, $9}'
```

## What this sprint is

S07 expands the trustworthy parse boundary from S06 into the four interfaces
that later pixel and volume stories consume. F-017 gives parsed attributes a
typed metadata model and provider registry. F-021 defines DICOMweb retrieval
while keeping fetch and authentication in the TypeScript shell. F-022 adds a
second medical-image input format without pretending its geometry conventions
are DICOM conventions. F-023 creates the explicit runtime codec registry that
later decoder stories populate.

This sprint establishes contracts and observable dispatch. It does not pull in
the image-plane and LUT modules of F-018, enhanced multiframe handling of
F-019, or the concrete codec implementations beginning with F-024.

## What is carried in

- **F-X011** remains pending because its acceptance evidence requires a second
  physical machine and none is available. It is unfinished M1 evidence, but it
  is not a dependency of any S07 story.
- **F-012 and F-014** provide the comparison and synthetic quirk-capture paths
  for ingest defects. They are completed foundations, not S07 story scope.

## The defect class this sprint is exposed to

**A plausible value can come from the wrong source or convention.** A metadata
model that collapses absent, empty, padded, and multi-valued attributes may
look correct on common files while losing information required by later LUT
and geometry code. Provider selection must remain observable, and absence must
not silently become a default supplied by another provider.

The source boundary is equally specific. HLD sections 3, 10, and 13 keep
DICOMweb fetch and authentication in TypeScript and declare `SeriesSource` as
the extension point for byte sources. Moving network ownership into Rust,
copying a retrieved instance across the boundary more than once, or exposing a
DICOMweb-specific type above the source contract would make the later DIMSE
entry point a rewrite.

NIfTI and DICOM do not name patient axes the same way. Treating a NIfTI affine
as if it were already DICOM patient geometry can produce a well-shaped but
mirrored or transposed volume. The input format and the coordinate conversion
must be explicit and independently tested with a non-symmetric affine.

The codec registry has the same silent-fallback risk as transfer-syntax
parsing. An unregistered Transfer Syntax UID must report unavailable rather
than selecting a common decoder. Capability answers must distinguish a known
syntax with no decoder from an unknown syntax, and registration order must not
silently change which decoder runs.

## What done means

- **F-017** defines the metadata types and provider lookup contract in
  `ocelli-dicom`. Tests preserve absence, padding, multiplicity, signed values,
  and the identity of the provider that answered.
- **F-021** implements the declared DICOMweb source path for WADO-RS,
  WADO-URI, and QIDO-RS. Fetch and authentication remain in TypeScript, bulk
  bytes cross into WebAssembly once, and HTTP or content-type failures remain
  distinct from DICOM parse failures.
- **F-022** validates NIfTI headers and payload bounds, preserves the declared
  affine, and converts its geometry into an explicit internal coordinate
  contract. Truncated input, unsupported datatype or endianness, and invalid
  dimensions are refused with synthetic fixtures.
- **F-023** implements explicit runtime codec registration through the HLD
  `Decoder` extension point. Lookup is by declared Transfer Syntax UID,
  unavailable capability is observable, and decoding writes into a
  caller-provided buffer without allocation.
- Native and wasm checks prove the same metadata, source, NIfTI, and codec
  contracts. `wasm-bindgen` remains confined to `ocelli-wasm`.
- Each dispatch or convention boundary has a controlled mutation observed red
  for the claimed reason. No tolerance or gate is weakened to make it pass.

## Dependency order

F-017, F-021, F-022, and F-023 all depend on F-016, which is done. No S07 story
is blocked by its declared dependencies.

The allocation declares no dependency among the four S07 stories. F-017,
F-021, and F-022 all touch `ocelli-dicom`, so their designs must settle shared
types and file ownership before concurrent implementation. F-023 primarily
owns `ocelli-codec`, but its Transfer Syntax UID contract must agree with the
observable dispatch established by F-016.

## Standing expectations

The HLD is authoritative. A design-plan departure is recorded in
`docs/hld/DEVIATIONS.md`, never improvised in implementation.

No patient data enters a prompt, tracked file, fixture, log, error or commit.
The ignored corpus remains behind `corpus/manifest.tsv` and its generators.

`SeriesSource` and `Decoder` are the declared extension-point exceptions that
may land with one implementer. No other new trait or generic parameter lands
without two users in the current tree.

Transfer-syntax, provider, and source capability are reported explicitly. An
unsupported route is unavailable or refused according to its contract and is
never silently treated as the common path.
