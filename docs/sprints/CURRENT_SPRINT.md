# Current sprint, S06

**Milestone**: M2, DICOM ingest and the pixel pipeline.
**Branch**: `sprint/s06`
**Opened**: 2026-09-07
**Goal**: Make `ocelli-dicom` parse DICOM Part 10 input and dispatch from the
declared transfer syntax over dicom-rs without silently choosing a different
encoding.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-016 | E3.1 | ocelli-dicom: parse and transfer-syntax dispatch over dicom-rs | Rust | 3w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together:

```bash
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S06 / {print $2, $9}'
```

## What this sprint is

S06 begins M2 at the first ingest boundary. F-016 turns the existing
`ocelli-dicom` scaffold into the parser named by HLD sections 4 and 28. It
accepts DICOM bytes, reads the Part 10 file meta information under its fixed
Explicit VR Little Endian encoding, resolves the declared Transfer Syntax UID,
and parses the following dataset under that syntax. This sprint establishes
the trustworthy parsed input that the metadata model, DICOMweb providers,
codec registry, pixel pipeline, and volume work consume later. It does not pull
those later stories into F-016.

## What is carried in

- **F-X011** remains pending because its acceptance evidence requires a second
  physical machine and none is available. It is an unfinished M1 evidence
  claim, but it is not a dependency of F-016 and does not block the start of
  M2.
- **F-012 and F-014** leave the comparison contract and synthetic quirk-capture
  path available for later ingest defects. They are completed foundations, not
  S06 story scope.

## The defect class this sprint is exposed to

**A parser can select a plausible but wrong encoding and still return a
dataset.** The file meta group is always Explicit VR Little Endian. Its
Transfer Syntax UID selects the encoding only for the dataset that follows.
Applying the selected syntax to the meta group, assuming Explicit VR Little
Endian for every dataset, or falling back after an unknown UID can all produce
credible attributes from common files while misreading another valid syntax.
The selected UID and the parser path must remain observable, and an unsupported
or malformed value must be refused rather than guessed.

The byte-order boundary is equally specific. Explicit VR Big Endian is retired
but present in the corpus, while Implicit VR Little Endian carries no VR on the
wire. Treating either as the common explicit little-endian case can preserve
reasonable tag numbers while corrupting lengths, values, or the later pixel
interpretation. Deflated Explicit VR Little Endian adds a second boundary
where successful inflation does not by itself prove the resulting dataset was
parsed under the right rules.

The parser also must not turn DICOM string conventions into host-language
assumptions. UI values may have NUL padding, other text VRs use space padding,
and DS or IS values can be multi-valued text. This story need not build the M2
metadata model, but its parsed representation must retain enough information
for that model to distinguish those cases later.

## What done means

- `crates/ocelli-dicom/src/parse.rs` owns the parsing entry point named by HLD
  section 28 and uses dicom-rs without introducing `wasm-bindgen` outside
  `ocelli-wasm`.
- Part 10 file meta information is read as Explicit VR Little Endian, and the
  declared Transfer Syntax UID determines the dataset parser path.
- Supported native, deflated, and encapsulated transfer-syntax cases reach the
  declared path. An unknown UID, malformed file meta information, truncated
  input, and a syntax that cannot run are explicit errors rather than fallback
  parses.
- Tests use the synthetic and licensed corpus behind `corpus/manifest.tsv` and
  include controlled negative cases. No patient data enters source, fixtures,
  logs, errors, documentation, or commits.
- A mutation that substitutes the common Explicit VR Little Endian path for a
  different declared syntax is observed red for the dispatch reason.
- Native and wasm checks demonstrate the same parser contract. Codec-specific
  decode capability remains the responsibility of F-023 and its dependent
  codec stories.

## Dependency order

F-016 depends on F-001, which is done. There is no blocked story in S06.

F-016 then unlocks F-017, F-021, F-022, and F-023 in S07. Those stories consume
the parsing boundary established here and do not need to be implemented to
close S06.

## Standing expectations

The HLD is authoritative. A design-plan departure is recorded in
`docs/hld/DEVIATIONS.md`, never improvised in implementation.

No patient data enters a prompt, tracked file, fixture, log, error or commit.
The ignored corpus remains behind `corpus/manifest.tsv` and its generators.

Transfer-syntax support is claimed only where a corpus case and an independent
expected result make the path observable. An unsupported syntax is reported as
unavailable or refused according to the design contract, never silently parsed
as another syntax.
