# F-017 review, pass 1

**Reviewed**: staged working tree against
`4e59cf22c6e1e307d69fb44bb01875b9b9e99803`, plus the approved design plan
**Result**: 3 defects, 0 smells, 0 nitpicks

## Defects

### D1, DICOM JSON carrier constructors admit states the standard forbids

**Where**: `crates/ocelli-dicom/src/metadata.rs:125`

**What**: `bulk_data_uri` accepts every `VR`, and `inline_binary` validates
only the base64 spelling while also accepting every `VR`. `InlineBinary::new`
accepts the empty string. A caller can therefore construct Inline Binary or
Bulk Data values for `PN`, and can encode a zero-length attribute as an empty
Inline Binary value.

**Why it is wrong**: DICOM PS3.18 F.2.2 permits `InlineBinary` only for `OB`,
`OD`, `OF`, `OL`, `OV`, `OW`, and `UN`. It permits `BulkDataURI` only for the
larger enumerated set in that section. PS3.18 F.2.5 represents a present empty
attribute by omitting `Value`, `BulkDataURI`, and `InlineBinary`, which is the
state this model already names `MetadataValue::Empty`. These public
constructors are the shared invariants F-021 is intended to use, so accepting
an invalid carrier and VR combination moves the validation gap into the next
story. Normative text:
<https://dicom.nema.org/medical/dicom/current/output/chtml/part18/sect_f.2.2.html>

**Evidence**: A disposable-copy integration probe passed after asserting that
`MetadataElement::inline_binary(VR::PN, "AA==")` returns `Ok` and that
`MetadataElement::bulk_data_uri(VR::PN, ...)` constructs a value. The shipped
unit test also lists `""` as a valid Inline Binary spelling. The probe command
was `cargo test -p ocelli-dicom --test review_probe`, which passed 2 tests.

### D2, duplicate function refusal depends on a comparison with no identity guarantee

**Where**: `crates/ocelli-dicom/src/provider.rs:102`

**What**: registration treats `std::ptr::fn_addr_eq` as a reliable identity
test for provider functions. Rust explicitly states that this comparison may
return true for distinct functions and may return false for the same function.
The registry can therefore refuse a distinct provider or accept a duplicate,
contrary to its public contract and approved plan.

**Why it is wrong**: F-017 requires duplicate registration of the same
function to be refused. Function pointers have callable behavior but no stable
intrinsic identity. A stable caller-supplied registration key can carry this
invariant, or the duplicate-function promise must be removed from the approved
contract. The Rust standard-library documentation records both false-positive
and false-negative cases:
<https://doc.rust-lang.org/std/ptr/fn.fn_addr_eq.html>

**Evidence**: `registry_refuses_duplicate_identity_and_duplicate_function`
passes in this build, but it checks only one linker result. The documented
guarantee for the primitive used at `provider.rs:113` says the comparison may
return false in any case and may merge equivalent distinct functions, so that
single observed address cannot establish the promised identity contract.

### D3, the generic trimmed view removes significant leading text

**Where**: `crates/ocelli-dicom/src/metadata.rs:256`

**What**: `MetadataValue::trimmed_text` strips spaces and NULL characters from
both ends without knowing the element VR. For `ST`, `LT`, and `UT`, leading
spaces are significant. The method returns a different semantic value for
those legal inputs.

**Why it is wrong**: DICOM PS3.5 section 6.2 gives different padding rules to
different text VRs. `ST`, `LT`, and `UT` allow ignorable trailing space padding
but state that leading spaces are significant. `DS` and `IS` permit leading
and trailing spaces. A semantic view cannot apply one bidirectional trim after
the VR has been separated from the value. Normative text:
<https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_6.2.html>

**Evidence**: A disposable-copy integration probe constructed
`MetadataValue::Text(vec!["  heading "])` and observed
`trimmed_text() == Some(vec!["heading"])`. The existing test covers only a
`DS` value, for which bidirectional space trimming is permitted, so it does
not expose the significant-leading-space case.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Read the full approved plan and the complete staged diff, including both
  manifests, the lockfile, public exports, metadata and provider modules,
  integration tests, and both LLD changes.
- Checked the implementation against DICOM PS3.5 section 6.2 and PS3.18
  sections F.2.2, F.2.5, and F.2.7. Inspected dicom-rs 0.10 collector,
  preserved-value decoder, primitive-value, and File Meta Information iterator
  source independently. The collector uses the preserved value strategy, and
  both main-data-set and file-meta strings retain their decoded source
  spelling for projection.
- `bin/ocelli.sh test ocelli-dicom` passed 58 tests with the corpus integration
  test ignored by its declared gate. The new metadata target contributed 11
  passing tests.
- A disposable mutation that made registry lookup continue after a present
  empty answer drove
  `ordered_provider_lookup_stops_on_present_empty_and_returns_identity` red.
  It failed with `left: None` and `right: Some("empty")`, so the claimed
  present-empty boundary is exercised.
- `bin/ocelli.sh check ocelli-dicom` and
  `bin/ocelli.sh clippy ocelli-dicom` passed. A wasm library check with
  `cargo check -p ocelli-dicom --lib --target wasm32-unknown-unknown` passed.
  The repository's native gate deliberately excludes `--all-targets` on
  wasm32 because the `proptest` development graph reaches `wait-timeout`, as
  documented directly in `bin/ocelli.sh`.
- No added `as` cast, `unsafe`, `wasm-bindgen`, pixel arithmetic, geometry
  arithmetic, render-loop path, or GPU submission appears in the staged diff.
  No new trait, generic abstraction, or dynamic dispatch was added.
- `python3 scripts/prose_check.py --staged`,
  `python3 scripts/source_provenance_check.py --staged`, and
  `git diff --cached --check` passed. The source-provenance check covered all
  9 staged files.
- Provider order, present-empty stopping, provider identity return, absent
  lookup, nested sequence item order, signed 16-bit and 32-bit values, padded
  UI and text spelling, ordered multi-valued DS text, and encapsulated-fragment
  refusal are each exercised by focused tests.
