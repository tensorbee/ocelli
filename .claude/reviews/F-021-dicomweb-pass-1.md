# F-021 review, pass 1

**Reviewed**: `f54b842364ed731fa465c91688dc9167c557a117` through the complete working tree, including modified and untracked files
**Result**: 11 defects, 0 smells, 0 nitpicks

## Defects

### D1, QIDO pagination accepts values outside the required unsigned-integer grammar

**Where**: `packages/core/src/dicomweb.ts:174`

**What**: `limit` and `offset` are stringified without validation. Negative,
fractional, `NaN`, and infinite values are fetched and committed.

**Why it is wrong**: DICOM PS3.18 2026c section 8.3.4 defines both values as
`uint`. The public `InvalidRequest` class exists for request parameters, and
invalid values must be refused before network I/O.

**Evidence**: A disposable Vitest probe supplied `-1`, `1.5`, `NaN`, and
positive infinity as `limit`. All four expected `InvalidRequest` cases failed
because the promises resolved after one fetch.

### D2, The positive QIDO fixture requires a forbidden repeated matching attribute

**Where**: `packages/core/src/dicomweb.test.ts:89`

**What**: The fixture supplies `00080060` twice and asserts both query values.
The client accepts the same repeated matching attribute generally.

**Why it is wrong**: DICOM PS3.18 2026c section 8.3.4.1 requires each
attribute in the query parameters not to be repeated. Repetition is permitted
for `includefield`. UID List matching uses one comma-separated value, not
repeated keys.

**Evidence**: The focused test passed while asserting
`getAll("00080060")` equals `['CT', 'MR']`. This makes the fixture positive
evidence for a non-conforming request.

### D3, Frame retrieval accepts unordered and repeated frame numbers

**Where**: `packages/core/src/dicomweb.ts:120`

**What**: The request check requires only positive safe integers. Lists such
as `[2, 1]` and `[1, 1]` are fetched.

**Why it is wrong**: DICOM PS3.18 2026c chapter 10 defines `{frames}` as a
comma-separated list of frame numbers in ascending order.

**Evidence**: Two disposable Vitest cases expected `InvalidRequest` before
fetch for descending and repeated lists. Both failed because the requests
resolved and committed.

### D4, Body-read failures escape the stable transport error boundary

**Where**: `packages/core/src/dicomweb.ts:203` and `packages/core/src/dicomweb.ts:226`

**What**: Only the fetch promise is caught. A rejection from
`response.arrayBuffer()` escapes as the upstream exception, so abort and
network failures during response consumption are neither classified nor
sanitized.

**Why it is wrong**: The approved plan and DICOMweb LLD say aborted operations
produce `Aborted`, rejected transport operations produce `Network`, and errors
never retain response or upstream text.

**Evidence**: A successful synthetic response whose `arrayBuffer()` rejected
with `DOMException('SYNTHETIC_SECRET_MARKER', 'AbortError')` returned that raw
DOMException. The probe failed its `DicomwebError`, `Aborted`, and no-marker
assertions.

### D5, Invalid configured URLs bypass the safe request error type and retain input

**Where**: `packages/core/src/dicomweb.ts:62`

**What**: Both base URLs are parsed directly in the constructor. An invalid
URL throws Node's native `TypeError`, whose `input` property retains the
supplied URL string.

**Why it is wrong**: The public class documentation, plan, and LLD state that
transport errors never retain URLs and define `InvalidRequest` for invalid
request parameters.

**Evidence**: Constructing a client with
`SYNTHETIC_SECRET_MARKER` as the Studies Service URL produced a native
`TypeError` with `input` equal to the marker. The disposable safety probe
failed.

### D6, An illegal outer boundary is copied before it is validated

**Where**: `packages/core/src/dicomweb.ts:267` and `packages/core/src/dicomweb.ts:226`

**What**: `requireMultipart` checks only that `boundary` exists. An illegal
boundary reaches `writeDicomwebResponse`, and Rust refuses it only after the
complete response has crossed the bulk boundary.

**Why it is wrong**: Approach step 6 requires TypeScript to validate the outer
Content-Type before any bulk write. The LLD repeats that claim and separately
states the 1 to 70 character boundary grammar.

**Evidence**: A response declaring `boundary="bad boundary "` resolved and
made one commit. The disposable test expecting `ContentType` and zero commits
failed.

### D7, Parameterized frame media types are rejected and transfer-syntax evidence is discarded

**Where**: `packages/core/src/dicomweb.ts:237`, `crates/ocelli-dicom/src/dicomweb.rs:194`, and `crates/ocelli-dicom/src/dicomweb.rs:197`

**What**: The TypeScript parser splits a quoted `type` parameter at every
semicolon. Rust compares only the base media type and stores only that base in
`EncodedFramePart`. A caller cannot successfully select a frame media type
with a `transfer-syntax` parameter, and the selected per-part Transfer Syntax
UID is not observable after parsing.

**Why it is wrong**: DICOM PS3.18 sections 8.7 and 10.4 define transfer syntax
as media-type negotiation evidence. The approved plan explicitly requires the
caller-selected frame media type and per-part media-type evidence.

**Evidence**: A valid quoted
`image/jls; transfer-syntax=1.2.840.10008.1.2.4.80` response failed in
TypeScript with `ContentType`. A direct Rust probe accepted the response but
returned only `image/jls` from `EncodedFramePart::media_type()`.

### D8, Multipart resource parts omit mandatory headers without refusal

**Where**: `crates/ocelli-dicom/src/dicomweb.rs:281`, `crates/ocelli-dicom/tests/dicomweb.rs:81`, and `docs/lld/dicomweb.md:84`

**What**: The parser requires only one `Content-Type`. It neither requires
`Content-Location` and `Content-Length` or `Transfer-Encoding`, nor checks that
a declared Content-Length matches the payload. The positive fixture itself
omits both length and transfer encoding.

**Why it is wrong**: DICOM PS3.18 2026c section 8.6.1.2.1 requires
Content-Type, Content-Location, and either Content-Length or Transfer-Encoding
for each represented resource part. The plan requires validation of each
part's headers.

**Evidence**: Disposable Rust tests expected `InvalidMultipart` for a part
with only Content-Type and for a five-byte payload declaring length four.
Both failed because parsing succeeded.

### D9, Legal string encodings of SV and UV are rejected

**Where**: `crates/ocelli-dicom/src/dicomweb.rs:488` and `crates/ocelli-dicom/src/dicomweb.rs:519`

**What**: SV and UV accept JSON numbers only.

**Why it is wrong**: DICOM PS3.18 2026c section F.2.3 permits SV and UV as a
JSON Number or String, including strings used to avoid precision loss.

**Evidence**: A disposable fixture using the minimum signed 64-bit value and
maximum unsigned 64-bit value as strings failed with
`InvalidJsonValue`.

### D10, Null slots are rejected for non-text multi-valued attributes

**Where**: `crates/ocelli-dicom/src/dicomweb.rs:425` and `crates/ocelli-dicom/src/metadata.rs:251`

**What**: Only the fallback text branch accepts JSON null. Numeric, AT, PN,
and other typed arrays reject it, and the F-017 value types have no way to
retain a typed empty slot.

**Why it is wrong**: DICOM PS3.18 2026c section F.2.5 represents empty values
inside any multi-valued attribute as null array elements. The plan promises
preserved Value multiplicity through the F-017 seam.

**Evidence**: A disposable JSON fixture containing `[1, null, 2]` for US and a
null second PN value failed with `InvalidJsonValue`.

### D11, Group Length attributes forbidden by the DICOM JSON model are accepted

**Where**: `crates/ocelli-dicom/src/dicomweb.rs:345`

**What**: Any uppercase eight-digit tag is inserted, including an element
number of `0000`.

**Why it is wrong**: DICOM PS3.18 2026c section F.2.2 says Group Length
attributes shall not be included in a DICOM JSON Model object.

**Evidence**: A disposable fixture containing `00080000` expected a parse
error. The test failed because the source returned a query batch containing
the attribute.

## Smells

None.

## Nitpicks

None.

## Verified clean

- `bin/ocelli.sh check ocelli-dicom` passed.
- `bin/ocelli.sh test ocelli-dicom --test dicomweb` passed 8 tests.
- `bin/ocelli.sh clippy ocelli-dicom` passed.
- `npm run typecheck` and `npm run lint` passed.
- `npx vitest run packages/core/src/dicomweb.test.ts packages/core/src/bulk.test.ts packages/core/src/index.test.ts`
  passed 20 tests in 3 files.
- The one-write mutation was observed red in a disposable copy. A second
  `commit_dicomweb_response` call made the named bulk test fail with 2 commits
  instead of 1.
- The boundary-anchor mutation was observed red in a disposable copy.
  Treating every boundary prefix as a delimiter made the boundary-like frame
  payload test fail.
- The mandatory WADO-URI parameter mutation was observed red in a disposable
  copy. Removing `objectUID` made the named request test fail with a missing
  value.
- URL path segments and query values use `URL` and `URLSearchParams`, and the
  WADO-URI operation supplies all four mandatory parameters plus
  `contentType=application/dicom` in the unchanged tree.
- Non-success HTTP status is checked before body consumption. The test proves
  the error body is not read, and status is retained without response text.
- The injected fetch owns authentication and receives the original
  `AbortSignal`. The unchanged tests also prove no retry after a rejected
  fetch.
- Multipart delimiter matching is anchored to CRLF plus the complete boundary
  and legal suffix. Frame payloads remain ranges into one owned response with
  no payload copy.
- WADO-URI and WADO-RS instance payloads reuse the F-016 Part 10 parser.
  `SeriesSource` is the HLD-declared extension-point exception and performs no
  I/O.
- Present empty attributes, absent attributes, text multiplicity, signed
  numeric widths, PN component groups, ordered SQ items, BulkDataURI, and
  InlineBinary were exercised through the F-017 public constructors. The
  separately reviewed F-017 sequence seam is used without an unchecked
  constructor.
- The F-101 deferral is honest. F-021 defines a fake sink boundary but adds no
  live Session binding, wasm export, or claim of end-to-end integration.
- The reviewed implementation adds no `unsafe`, `wasm-bindgen`, WebAssembly
  view construction outside `bulk.ts`, pixel arithmetic, render-loop path, or
  second source implementation.
- The dependency and build-target documentation agree that `serde_json` is a
  direct `ocelli-dicom` dependency on native and wasm targets.
- `git diff --check` passed before this report was added.
