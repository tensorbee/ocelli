# F-021 review, pass 2

**Reviewed**: `f54b842364ed731fa465c91688dc9167c557a117` through the exact staged 21-file working tree
**Result**: 4 defects, 0 smells, 0 nitpicks

## Defects

### D1, Unsupported current QIDO controls can escape through the generic filter surface

**Where**: `packages/core/src/dicomweb.ts:177`

**What**: The generic matching-attribute filter rejects `includefield`,
`limit`, `offset`, and `fuzzymatching`, but does not reserve
`emptyvaluematching` or `multiplevaluematching`. Callers can therefore send
both current search controls as if they were matching attributes.

**Why it is wrong**: DICOM PS3.18 2026c section 8.3.4 defines these names as
search control parameters, not matching attributes. The approved F-021 API
deliberately exposes typed options only for `includefield`, `limit`, `offset`,
and `fuzzymatching`. The two additional controls do not need new public typed
options in this story, but they must be reserved and refused by the generic
filter path.

**Evidence**: A disposable Vitest probe supplied each name through `filters`
and expected `InvalidRequest` before fetch. Both promises resolved and each
performed a fetch instead.

### D2, The QIDO LLD falsely says matching filters use repeated keys

**Where**: `docs/lld/dicomweb.md:25`

**What**: The LLD says QIDO filters and `includefield` use repeated query keys.
The remediated client correctly rejects a repeated matching attribute and only
allows `includefield` to repeat.

**Why it is wrong**: DICOM PS3.18 2026c section 8.3.4.1 requires each matching
attribute not to be repeated. A UID List is one comma-separated value. The
approved plan's bounded option list supports filters plus the four named typed
controls, but its phrase about not collapsing repeated query keys is only
correct for `includefield`. The LLD must state that distinction explicitly.

**Evidence**: The staged positive test sends one value for each matching
attribute and multiple `includefield` values. Its negative test rejects a
duplicate matching tag before fetch, directly contradicting the LLD sentence.

### D3, Multipart parsing accepts whitespace before a required header colon

**Where**: `crates/ocelli-dicom/src/dicomweb.rs:303`

**What**: `parse_part_headers` trims the field name after splitting on the
colon. A line such as `Content-Type : application/dicom` is therefore accepted
as the mandatory Content-Type header.

**Why it is wrong**: The DICOM PS3.18 2026c section 8.6.1.2.1 multipart
resource grammar requires the ordered header field syntax. Whitespace is not
part of the field name and cannot be normalized into a valid required header.

**Evidence**: A disposable Rust fixture used whitespace before the first
required header colon and expected `InvalidMultipart`. Framing succeeded and
the response reached Part 10 parsing, which returned `TruncatedPart10`.

### D4, Multipart parsing does not validate Content-Location as a URI

**Where**: `crates/ocelli-dicom/src/dicomweb.rs:313`

**What**: The parser checks only that Content-Location is nonempty. A value
containing spaces, `not a URI`, is accepted as the mandatory resource location.

**Why it is wrong**: DICOM PS3.18 2026c section 8.6.1.2.1 requires a
Content-Location URL for each represented resource. Nonempty arbitrary text
does not satisfy that grammar.

**Evidence**: A disposable Rust fixture supplied `Content-Location: not a URI`
and expected `InvalidMultipart`. Framing succeeded and the response reached
Part 10 parsing, which returned `TruncatedPart10`.

## Smells

None.

## Nitpicks

None.

## Verified clean

- All pass-1 defects D1 through D11 were revisited against the staged source,
  tests, the canonical DICOM expert references, and DICOM PS3.18 2026c.
- `bin/ocelli.sh test ocelli-dicom` passed 75 tests with the one corpus test
  ignored as declared.
- `bin/ocelli.sh check ocelli-dicom` and
  `bin/ocelli.sh clippy ocelli-dicom` passed.
- `bin/ocelli.sh native` passed all four native and wasm32 cross-target steps.
- `npx vitest run packages/core/src/dicomweb.test.ts packages/core/src/bulk.test.ts packages/core/src/index.test.ts`
  passed 23 tests in 3 files.
- `npm run typecheck`, `npm run lint`, and `bin/ocelli.sh fmt --check` passed.
- `python3 scripts/prose_check.py` passed over 232 files.
- `bin/ocelli.sh gate content` and `git diff --cached --check` passed before
  this review artifact was added.
- Disposable positive probes confirmed order-independent quoted MIME
  parameters, retention of the full per-part media type and transfer syntax,
  ordered required multipart headers, exact ASCII Content-Length handling,
  all-null PN and AT positions, and empty-object SQ item semantics.
- The staged client refuses empty, descending, and repeated frame lists before
  fetch. A mutation changing the strict comparison from `<=` to `<` made the
  repeated-frame test fail.
- Fetch and response-body `arrayBuffer` failures are classified separately as
  aborted or network failures without retaining upstream text. A mutation
  collapsing response-body aborts into `Network` made the sanitizer test fail.
- QIDO `limit` and `offset` enforce the unsigned-integer domain. A mutation
  accepting every value made the invalid-pagination test fail.
- DICOM JSON SV and UV string encodings map to typed signed and unsigned 64-bit
  values. Typed null positions for text, US, PN, and AT remain observable, and
  all-null values retain multiplicity.
- Group Length attributes are refused recursively by the DICOM JSON parser. A
  mutation removing the element-zero filter made the invalid-structure test
  fail.
- The one-write boundary remains sensitive. A disposable second
  `commit_dicomweb_response` call made the named bulk test fail with two
  commits instead of one.
- No positive Rust fixture uses a silent `let else` exit without first
  asserting the expected result shape.
- WADO-RS, WADO-URI, and QIDO-RS use caller-owned fetch authentication and
  cancellation, stable safe error classes, and no automatic retry.
- Encoded multipart frames remain ranges into one owned response allocation.
  Instance parts reuse the F-016 Part 10 parser, and QIDO JSON maps into F-017
  metadata values through the checked NullSlots seam.
- F-101 remains an explicit deferral for the live Session command and wasm
  commit binding. The staged implementation makes no end-to-end integration
  claim.
- The reviewed tree adds no `unsafe`, `wasm-bindgen` outside `ocelli-wasm`,
  pixel arithmetic, render-loop work, second source implementation, or
  uninstantiated generic.
