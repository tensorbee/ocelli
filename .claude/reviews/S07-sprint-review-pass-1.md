# S07 sprint review, pass 1

**Reviewed**: full sprint diff from
`649af7ed05b1c5e42bfa1345f39e15eef07dc735` through
`9051830199d08bb49d4eb817704ddc49356a623d`, exact combined staged and HEAD
tree `784dbed779ec3b971f90dc3777d981d073b8a588`
**Result**: 4 defects, 0 smells, 0 nitpicks

## Defects

### D1, The retained High Bit 15 corpus case is presented as conforming while F-023 correctly refuses it

**Where**: `corpus/manifest.tsv:5`, `scripts/corpus_synth.py:387`,
`scripts/tests/test_corpus_synth.py:65`, `corpus/README.md:75`,
`docs/lld/corpus.md:31`, `.claude/skills/dicom-tooling/SKILL.md:357`, and
`crates/ocelli-codec/src/registry.rs:141`

**What**: The corpus retains `synthetic/ct_signed_12in16_left.dcm` with Bits
Stored 12 and High Bit 15. Its manifest token, generator, hand-computed fixture
comment, corpus documentation, LLD, and canonical tooling skill present it as
the left-aligned peer of the conforming High Bit 11 case. None identifies it as
legacy or nonconforming input. F-023 instead enforces High Bit equal to Bits
Stored minus one and therefore refuses this exact descriptor.

**Why it is wrong**: Current DICOM PS3.3 C.7.6.3.3 requires High Bit to be one
less than Bits Stored. The approved F-023 plan transcribes that requirement and
explicitly says that a shifted stored-bit window is not a conforming Image
Pixel Description Macro. The new codec LLD says the same. The combined sprint
therefore gives incompatible meanings to one of its retained conformance
fixtures. A later decoder can follow the registry contract and reject the case
while corpus prose and generated oracle expectations continue to call that a
failure.

**Evidence**: A current-tree probe constructed the exact relevant descriptor,
16 Bits Allocated, 12 Bits Stored, High Bit 15, and signed representation.
`FrameDesc::new` returned
`HighBitMismatch { high_bit: 15, expected: 11 }`. The manifest still names the
row `high-bit-15`, and the generator says that real scanners produce both
alignments without marking the second one nonconforming. Existing F-023 tests
exercise other mismatches but do not bind this retained row to its required
refusal.

Keep the row as a legacy interoperability input. Mark it consistently as
nonconforming in the manifest category, generator, corpus documentation, LLD,
and canonical tooling skill. Add an executable cross-story test proving that
the retained High Bit 15 descriptor is refused with expected High Bit 11. Do
not weaken F-023's equality or delete the legacy evidence.

### D2, The explicit 16-UID codec catalogue has no executable agreement with the corpus and ingest surfaces

**Where**: `crates/ocelli-codec/src/registry.rs:15`,
`scripts/corpus_check.py:41`, `crates/ocelli-dicom/src/parse.rs:212`,
`crates/ocelli-dicom/src/dicomweb.rs:76`, `docs/lld/codecs.md:22`, and
`docs/sprints/CURRENT_SPRINT.md:100`

**What**: F-023 and the corpus checker independently hard-code the same 16
Transfer Syntax UIDs. F-023 tests prove only that its local slice contains 16
distinct nonempty strings and that each starts known but unavailable. F-016
resolves its parseable data-set syntaxes from the pinned dicom-rs registry, and
F-021 retains a frame part's declared Transfer Syntax UID as a string. No test
connects those three observable contracts or proves that the Rust catalogue is
the exact corpus catalogue.

**Why it is wrong**: The approved F-023 plan intentionally defines a 16-UID
product and corpus surface. The codec LLD says the catalogue is shared with the
corpus, and the active sprint says its UID contract must agree with F-016's
observable dispatch. Those are cross-story acceptance claims, not local
collection-shape claims. The broader F-016 parser surface is not itself an
error. A current dependency scan found 44 parseable data-set syntaxes, of which
the intended 16 are a subset. The defect is that the intended subset and its
overlap have no executable authority, so a valid-looking substitution can
change product capability while every current owner still passes its isolated
tests.

**Evidence**: In a disposable copy of this exact tree, replacing the final
Rust catalogue entry, HTJ2K `1.2.840.10008.1.2.4.203`, with parser-recognized
MPEG2 `1.2.840.10008.1.2.4.100` left all 14 `ocelli-codec` tests green. The
unchanged corpus checker then also reported `transfer syntaxes: 16 of 16` and
`OK: coverage complete` across 92 rows. A current-tree synthetic Part 10 probe
with MPEG2 UID `1.2.840.10008.1.2.4.100` parsed through F-016 as
`DispatchPath::Encapsulated`, while F-023 correctly reported `Unknown` because
MPEG2 is outside the selected codec surface. That result confirms why agreement
must mean an explicit selected subset, not equality with all 44 parser entries.

Preserve the approved 16-UID subset. Establish one executable canonical-set
proof that the Rust catalogue exactly matches the corpus registry surface and
that every selected UID is accepted by F-016's observable dispatch. Add a
focused F-021 seam case showing that an exact selected frame UID survives the
multipart boundary and maps to `KnownUnavailable`. The mutation above must
make that proof red. Clarify the sprint wording so it does not imply that all
parser-recognized syntaxes belong in the codec catalogue.

### D3, Three S07 implementation commits have no verification or generator provenance trailers

**Where**: commits `f40c719550e860e8632194241d021148ffc9483b`,
`f23b8d22e0c91d72541de015eb9be5aa31dadc11`, and
`f54b842364ed731fa465c91688dc9167c557a117`

**What**: The F-017 integration commit, F-023 integration commit, and later
F-017 sequence-seam commit carry neither `Ocelli-Verify` nor
`Ocelli-Generated-By`. Their messages otherwise use the feature integration
format and claim prepared verification, but omit the mechanical evidence that
binds a passing ledger record and named generator to the committed tree.

**Why it is wrong**: HLD 27.2 R6 requires a provenance trailer on every
commit. The workflow requires both trailers, writes them from the verification
ledger through the pre-commit hook, and makes `check-commit` the CI proof that
the named tree was actually verified. These are implementation commits in the
reviewed sprint range, not exempt scaffolding outside the workflow.

**Evidence**: `git show -s --format=fuller` displays no trailers on any of the
three commits. `python3 scripts/verify_ledger.py check-commit` failed each one
with `carries no Ocelli-Verify trailer`. The absence of
`Ocelli-Generated-By` is also visible in each complete message.

Commits are immutable, so this cannot be repaired by editing a tracked record
on top. Because the branch is reported as unpublished, the normal repair is a
deliberate local history rewrite that recreates each affected commit through
the ledger and hook against its exact tree. Every descendant commit hash will
change and all reviewed tree references must then be regenerated. The trailers
must not be hand-written. If history may not be rewritten, an explicit approved
HLD deviation is required before close, with the permanent provenance gap and
its consequences recorded.

### D4, S07 planning, LLD, and AS_BUILT records disagree with the delivered ownership and dependency surface

**Where**: `.claude/plans/F-021-design.md`,
`docs/lld/typescript-packaging.md`, `docs/lld/build-targets.md`, and
`docs/sprints/AS_BUILT.md`

**What**: Five current delivery claims are false or mutually inconsistent.

- The F-021 plan authorizes its scoped `NullSlots` additions in F-017's
  `metadata.rs` and `metadata.rs` test surface, then concludes that F-021 does
  not edit F-017's metadata implementation. The delivered commit did edit both
  files, and the F-017 AS_BUILT entry assigns that later addition to F-021.
- The TypeScript packaging LLD still describes the pre-F-021 public package
  surface and does not account for the delivered DICOMweb module and exports.
- The build-target LLD's contributor and dependency account omits F-017 and
  the delivered `dicom-core` dependency edge.
- The F-021 AS_BUILT deviation field says none even though its approved plan
  explicitly applies existing deviations D-02 and D-18.
- The F-017 AS_BUILT test inventory counts provider tests, sixteen initial
  metadata integration and property tests, and the later sequence regression,
  but omits the `metadata.rs` Inline Binary unit test delivered by F-017.

**Why it is wrong**: These records are the repository's shared ownership,
target, dependency, and audit trail. The sprint marks all four stories done,
so a later worker is entitled to use them to determine which story owns a
public type, why a dependency is present, what the TypeScript package exports,
which deviations govern it, and which tests form the recorded evidence. The
current answers disagree with the exact tree and with one another.

Reconcile the F-021 plan's closing ownership sentence with its approved scoped
seam write set. Update the packaging and build-target LLDs to the delivered
module, export, contributor, and dependency surface. Preserve AS_BUILT's
append-only history by adding explicit F-017 and F-021 correction entries for
the omitted unit test and existing deviations rather than rewriting the
original completion entries.

## Smells

None.

## Nitpicks

None.

## Verified clean

- The complete 51-file sprint diff and all four approved plans were reviewed,
  together with every F-017, F-021, F-022, and F-023 feature review artifact,
  relevant HLD and LLD sections, current sprint records, and the canonical
  DICOM expert and tooling skills.
- F-017 preserves absent versus present-empty values, exact VR and
  multiplicity, sequence item order, typed signed values, DICOM JSON carrier
  distinctions, and provider identity. The provider registry's ownership and
  first-match behavior agree with the approved plan.
- F-021 keeps fetch, authentication, status handling, and content negotiation
  in TypeScript. Rust owns byte validation and parsing. Bulk response bytes
  cross the sink once, multipart frame views retain one owned response, and
  transport, media-type, and DICOM parse failures remain distinct without
  exposing response bodies.
- F-022's header offsets, endian reads, datatype widths, dimension products,
  payload bounds, unit scaling, exact float offset rule, and rank refusal were
  checked against the plan. Qform mixed terms, sform column order, qform and
  sform selection, and the left-multiplied RAS-to-LPS conversion are covered by
  non-symmetric fixtures. No tolerance is used for header or affine selection.
- F-023's registry uses exact UID lookup, distinguishes unknown from known but
  unavailable, preflights multi-UID registration atomically, refuses
  collisions, retains shared decoder ownership, validates checked output size,
  and passes caller-owned source and destination buffers directly to the
  selected decoder.
- No new pixel arithmetic or concrete codec is hidden in F-023. The
  `decode.frame` benchmark remains honestly unavailable until a real adapter
  exists. No `wasm-bindgen` escaped `ocelli-wasm`, no new unsafe block landed,
  and target-specific codec registration was not introduced.
- The four story status rows and tracker rows agree that implementation is
  done. D3 and D4 identify the provenance and delivery-record claims that do
  not yet agree with the exact history and delivered surface.
- The already-recorded consolidated sprint gate passed all 30 gates on the
  reviewed tree. It verified 92 corpus rows, 99 browser oracle views with
  identical hashes, and all 29 oracle mutations. This pass did not replay that
  full gate. Its two focused probes demonstrate seams that those green results
  do not cover.
- The exact High Bit probe returned the required F-023 mismatch, and the
  disposable UID substitution left all 14 codec tests and the 92-row
  manifest-only corpus coverage check green. No repository implementation or
  documentation was changed by either probe.
- Direct commit inspection and `verify_ledger.py check-commit` independently
  confirmed the three missing-trailer failures in D3.
