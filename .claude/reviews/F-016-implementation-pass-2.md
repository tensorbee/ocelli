# F-016 review, pass 2

**Reviewed**: remediated working tree relative to `2f0de7c`, including
untracked files
**Result**: 2 defects, 0 smells, 0 nitpicks

## Defects

### D1, The completion sentinel collides with input data

**Where**: `crates/ocelli-dicom/src/parse.rs:26`

**What**: Completion is proved only by finding tag `(0002,0000)` in the
collected main data set. The input can contain that same tag. dicom-rs collects
elements into a `BTreeMap` and replaces an earlier element when the same tag
appears again. A complete input occurrence is therefore silently discarded
when the appended sentinel is parsed. More seriously, a complete input
occurrence remains available to `object.take(COMPLETION_SENTINEL)` when a
later truncated value consumes all twelve appended sentinel bytes. That input
returns `Ok` even though the real sentinel never became an element.

**Why it is wrong**: PS3.10 section 7 places group 0002 in File Meta
Information, not in the main Data Set. A strict parser must refuse such an
input element instead of deleting it. The approved plan also promises the
complete dicom-rs object and a truncated-data-set refusal. A marker whose
presence can come from the untrusted input proves neither claim.

**Evidence**: `append_completion_sentinel` appends a twelve-byte
`(0002,0000) UL` element and line 186 tests presence only. The pinned
dicom-object 0.10 `Extend<InMemElement>` implementation extends its tag-keyed
map, which replaces duplicate tags. An explicit-VR input can place its own
complete `(0002,0000) UL` before an `OB` element declaring twelve value bytes
but providing none. The appended sentinel becomes that `OB` value, EOF is
then clean, and the input occurrence satisfies the presence test.

### D2, The remediated design plan still names code that no longer runs

**Where**: `.claude/plans/F-016-design.md:194`

**What**: Approach step 4 still says the remaining bytes are parsed with
`InMemDicomObject::read_dataset_with_ts`. The remediation replaced that call
with explicit data-set adaptation followed by `DicomCollectorOptions` and
`read_dataset_to_end`.

**Why it is wrong**: The plan is a tracked factual record of the approved
implementation. The microscope rules treat a false prose claim as a defect.
Keeping both parser descriptions in one step also obscures the strict odd
length and completion behavior that motivated the remediation.

**Evidence**: `rg -n "read_dataset_with_ts" crates/ocelli-dicom/src
.claude/plans/F-016-design.md` finds the name only in the plan. The working
implementation uses `adapt_data_set`, `DicomCollectorOptions`, and
`read_dataset_to_end` at lines 174 through 185.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-1 D1 is closed. D-18 and the plan now record the required `std`
  posture, `scripts/no_std_check.py` names the resulting eight-crate no_std
  set, and `bin/ocelli.sh gate nostd` passes.
- Pass-1 D2 is closed for the exercised structural failures. Collector errors
  with an `UnexpectedEof` source map to `TruncatedDataSet`, other structural
  failures map to `InvalidDataSet`, and the item-delimiter-first fixture
  reaches the latter.
- Pass-1 D3 is closed for ordinary inputs. The completion marker exposes one,
  two, and three trailing bytes after a complete top-level data set. It also
  makes a missing encapsulated Pixel Data sequence delimiter fail. D1 above
  is the remaining collision that prevents this mechanism being complete.
- Pass-1 D4 is closed. The collector is configured with
  `OddLengthStrategy::Fail`, and an independently encoded odd-length fixture
  is refused as `InvalidDataSet`.
- The completion element has the correct implicit encoding, Explicit VR
  Little Endian encoding, and Explicit VR Big Endian encoding. Deflated data
  is adapted once before the Explicit VR Little Endian marker is appended.
  The ordinary route fixtures exercise successful marker collection across
  all five dispatch paths.
- The collector removes the appended marker before attaching the exact File
  Meta Information. String trimming, multiplicity, checked integer values,
  transfer-syntax evidence, and encapsulated fragments remain available.
- `bin/ocelli.sh test ocelli-dicom` passed 19 executed tests, with the local
  corpus test ignored in the ordinary suite.
- `bin/ocelli.sh gate corpus` passed over 92 verified rows and all 16 declared
  transfer syntaxes. Digest and metadata verification still precede the Rust
  parser test through fail-closed `&&` chaining.
- `bin/ocelli.sh check ocelli-dicom`, `bin/ocelli.sh clippy ocelli-dicom`, and
  `bin/ocelli.sh native` passed. The latter includes the wasm32 build and
  cross-target feature comparison.
- `bin/ocelli.sh gate deviations` and `python3 scripts/prose_check.py` passed.
- The remediation adds no `unsafe`, `wasm-bindgen`, pixel decode, rendering
  path, trait, generic parameter, or dynamic dispatch.
