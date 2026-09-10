# F-017 review, pass 2

**Reviewed**: fully staged working tree against
`4e59cf22c6e1e307d69fb44bb01875b9b9e99803`, the approved design plan, and
the pass-1 report
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Read the complete approved F-017 plan, pass-1 report, microscope workflow,
  canonical DICOM expert skill, and the complete staged diff. The diff contains
  1,430 insertions and 18 deletions across 11 files. It includes both manifests,
  the lockfile, public exports, metadata and provider modules, integration
  tests, the amended plan, the pass-1 report, and both LLD updates.
- Verified D1 against DICOM PS3.18 F.2.2 and F.2.5. `BulkDataURI` accepts
  exactly `DS`, `FL`, `FD`, `IS`, `LT`, `OB`, `OD`, `OF`, `OL`, `OV`, `OW`,
  `SL`, `SS`, `ST`, `SV`, `UC`, `UL`, `UN`, `US`, `UT`, and `UV`.
  `InlineBinary` accepts exactly `OB`, `OD`, `OF`, `OL`, `OV`, `OW`, and
  `UN`. Disallowed VRs return typed structural errors. Empty Inline Binary
  returns `EmptyInlineBinary`, while a present zero-length attribute remains
  representable as `MetadataValue::Empty` without a JSON carrier.
- Verified the Inline Binary validator rejects malformed padding, non-base64
  bytes, and non-zero unused bits while retaining valid canonical padded
  base64. A disposable mutation that removed `OB` from the Inline Binary
  allow-list drove `inline_binary_accepts_every_ps318_f22_vr` red at `OB`.
  A separate mutation that admitted empty Inline Binary drove
  `inline_binary_rejects_empty_and_invalid_base64` red with `left: None` and
  `right: Some(EmptyInlineBinary)`.
- Verified D2. Registration compares only caller-supplied `ProviderId` values
  and does not compare function addresses. A repeated ID is refused even when
  the function differs. The same function is accepted under a distinct ID.
  Lookup returns the answering ID, follows registration order, and stops on a
  present empty element. A disposable mutation that restored function-address
  comparison drove
  `registry_uses_provider_id_as_the_only_registration_identity` red because a
  repeated ID with a different function was incorrectly accepted.
- Verified D3 against DICOM PS3.5 section 6.2. The semantic text view retains
  source spelling and applies VR-specific padding semantics. It trims both
  ends for `AE`, `CS`, `DS`, `IS`, `LO`, `PN`, and `SH`, removes only trailing
  NULL padding for `UI`, and otherwise removes only trailing space padding.
  Significant leading ST space is preserved. A disposable mutation that used
  bidirectional trimming for the trailing-pad-only branch drove
  `projection_distinguishes_missing_empty_and_preserves_wire_spelling` red
  with the two leading spaces removed.
- Verified the remaining metadata surface. Missing and present-empty states
  remain distinct. Declared VR, ordered multiplicity, signed and unsigned
  widths, floating widths, tags, byte and word payloads, partial date and time
  values, nested sequence item order, Person Name component groups, padded
  source spelling, and encapsulated-fragment refusal each remain explicit.
  No numeric coercion, lossy cast, pixel arithmetic, or geometry arithmetic is
  introduced.
- Verified the provider surface. The registry is caller-owned, has no implicit
  provider or merge behavior, and supplies distinct main-data-set and File
  Meta Information providers through one result type. The implementation adds
  no trait, generic extension point, dynamic dispatch, unsafe block,
  `wasm-bindgen` dependency, render-loop work, or GPU submission.
- `bin/ocelli.sh test ocelli-dicom` passed 63 tests with 0 failures. The one
  corpus integration test remained ignored behind its declared corpus gate.
  The metadata integration target contributed 16 passing tests, including the
  complete carrier allow-lists, invalid carrier cases, empty semantics,
  semantic ST text, provider identity, signedness, multiplicity, nesting, and
  four property tests.
- `bin/ocelli.sh check ocelli-dicom`, `bin/ocelli.sh clippy ocelli-dicom`, and
  `bin/ocelli.sh wasm` passed. The release wasm build completed through
  `wasm-opt` without introducing `wasm-bindgen` outside `ocelli-wasm`.
- Dependency declarations and the lockfile agree. `cargo tree -p ocelli-dicom
  --edges normal,dev --depth 1` reports direct `dicom-core` 0.10 and the
  existing dicom-rs components as normal dependencies, with `proptest` 1.11 as
  the sole new development dependency. Default dicom-rs features remain
  disabled in the workspace manifest.
- The approved plan and `docs/lld/dicom-ingest.md` now state the same carrier
  VR sets and empty rule, VR-aware text behavior, caller-supplied ProviderId
  identity, provider precedence, dependency edge, tests, and refusal boundary.
  `docs/lld/README.md` indexes F-017 against the ingest LLD.
- `python3 scripts/prose_check.py --staged`, `python3
  scripts/source_provenance_check.py --staged`, and `git diff --cached
  --check` passed. The provenance check covered all 11 staged files before
  this pass-2 report was added.
