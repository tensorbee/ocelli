# F-021, DICOMweb client: WADO-RS, WADO-URI, QIDO-RS

**Status**: approved
**Epic ref**: E3.6
**Sprint**: S07
**Estimate**: 3w

## Normative source, transcribed

_The quotations below normalise the source's em dash to a hyphen and its prose
semicolon to a full stop because `scripts/prose_check.py` covers this plan. No
word is changed. The tracked HLD wins where exact bytes matter._

### `docs/hld/03-architecture-and-crates.md`, sections 3 and 4

> The shell is TypeScript on the main thread: DOM and pointer events, the SVG
> annotation layer, tool interaction state, framework bindings, and DICOMweb
> fetch and authentication. The core is Rust running in workers. Between them
> sits a deliberately narrow boundary carrying commands down, bulk bytes down,
> and events up.

| **Crate** | **Responsibility** | **wasm** | **native** |
|----|----|----|----|
| ocelli-dicom | Parsing, transfer-syntax dispatch, metadata model and providers | yes | yes |
| ocelli-wasm | The only crate that may import wasm-bindgen. Boundary, commands, event ring. | yes | no |

### `docs/hld/04-boundary-and-data-path.md`, section 5.2

> The core allocates and returns a pointer and length. JavaScript builds a
> typed-array view immediately before writing and discards it after. Views are
> never cached across a call that might allocate, because any WebAssembly
> memory growth relocates the backing buffer and detaches every outstanding
> view. This is the sharpest edge in the whole design and section 17.2 gives
> the exact pattern.

### `docs/hld/07-concurrency-and-typescript.md`, sections 9 and 10

> **Start single-threaded per worker.** N decode workers, each holding its own
> WebAssembly instance. One render worker owning the GPUDevice and every
> OffscreenCanvas. The main thread doing DOM, events and tool UI. No
> SharedArrayBuffer, no wasm-bindgen-rayon, no nightly toolchain.

> - Framework bindings, DICOMweb fetch and authentication

### `docs/hld/10-extension-points.md`, section 13

> - **A SeriesSource trait** abstracts where bytes come from. Phase 1
> implements DICOMweb. Phase 2 adds a DIMSE-backed implementation without
> touching anything above it.

### `docs/hld/12-workspace-and-build.md`, Part II and section 15.1

> This part is prescriptive. Where it gives a formula, a layout or a
> signature, that is the intended implementation and a deviation should be
> raised rather than improvised. It exists because the dangerous defect in
> medical imaging is not the crash - it is the pixel that is quietly wrong,
> and quietly wrong code is produced by reasonable people making locally
> reasonable choices.

> ocelli-dicom/ \# parse, metadata, providers

> ocelli-wasm/ \# \*\* the only wasm-bindgen crate \*\*

> packages/
>
> core/ \# @ocelli/core (TypeScript shell)

### `docs/hld/14-the-boundary-in-code.md`, section 17.2

```typescript
#[wasm_bindgen]
impl Session {
    /// Reserve `len` bytes and return a pointer into linear memory.
    pub fn alloc(&mut self, len: usize) -> *mut u8 { /* ... */ }
    /// Hand ownership back; `ptr` must be the value returned by `alloc`.
    pub fn commit_frame(&mut self, ptr: *mut u8, len: usize, meta: &FrameMeta) { /* ... */ }
}
// packages/core/src/bulk.ts -- the ONLY correct order
const ptr = session.alloc(bytes.byteLength);
// Build the view AFTER the allocation. Use it immediately. Let it go.
new Uint8Array(wasm.memory.buffer, ptr, bytes.byteLength).set(bytes);
session.commit_frame(ptr, bytes.byteLength, meta);
```

> **NEVER CACHE THE VIEW** A module-level const HEAP = new
> Uint8Array(wasm.memory.buffer) is the classic failure. Any wasm memory growth
> relocates the ArrayBuffer and detaches every outstanding view. The next write
> silently targets a detached buffer or throws far from the cause. Add an
> ESLint rule banning new Uint8Array(wasm.memory.buffer) outside the two
> functions that are allowed to do it.

### `docs/hld/20-errors-and-panics.md`, section 23

> thiserror in the core crates. The boundary maps everything to a stable
> numeric code and a message. The important part is what happens when that is
> not enough.

> - Error codes are stable and versioned. The shell switches on the code. The
> message is for humans and may change.

F-021 keeps HTTP, authentication, content negotiation, and cancellation errors
on the TypeScript side. Rust `SourceError` variants cover response structure,
DICOM JSON, and Part 10 parsing. There is no live `Session` producer in this
story, so F-021 does not reserve a numeric boundary error code.

### `docs/hld/21-worker-protocol.md`, section 24

```text
main -> decode : { kind:'decode', seriesId, frameIndex, buffer } [transfer]
decode -> render : { kind:'frame', frameId, buffer, meta } [transfer]
main -> render : { kind:'commands', buffer } [transfer]
render -> main : drained event ring [copy]
// Always transfer, never copy:
// worker.postMessage(msg, [msg.buffer]);
```

> Three roles: the main thread, N decode workers each with its own WebAssembly
> instance, and one render worker owning the GPUDevice and every
> OffscreenCanvas. Decode workers never touch the GPU. The render worker never
> decodes.

### `docs/hld/23-performance-rules.md`, section 26

> - No allocation in the render loop.
>
> - Measure with the benchmark harness before optimising anything. The
> intuitions that work in JavaScript do not transfer.

### `docs/hld/24-agent-code-standards.md`, sections 27.2 and 27.3

| **#** | **Rule** | **Why** |
|----|----|----|
| R2 | Tests derive from the spec or the oracle, never from reading the implementation | An agent asked to test a function will assert what it does, not what it should do |
| R5 | No unsafe outside the allow-list (ocelli-wasm/src/ring.rs, ocelli-core/src/cast.rs) | Keeps the audit surface to two files |

> That a new test would actually fail if the code were wrong. Mutate one
> constant, re-run, confirm it goes red.

### `docs/hld/26-differentiating-capabilities.md`, section 35

> - **Frame-level DICOMweb retrieval** - pull the individual frames that fill
> viewport tiles, never whole instances. Mandatory at gigapixel scale.

F-021 includes the frame-resource request and response shape now. It does not
implement the Pyramid source, tile scheduler, cache policy, or WSI viewport.

### DICOM PS3.18 constraints used by this plan

- PS3.18 2026c section 10.6 defines QIDO-RS as GET searches over study,
  series, and instance resources. The JSON response is
  `application/dicom+json`.
- PS3.18 2026c section 10.4 defines WADO-RS instance resources at
  `/studies/{study}/series/{series}/instances/{instance}` and frame resources
  below `/frames/{frames}`. Instance and frame REST responses may be
  `multipart/related`, including a one-part response.
- PS3.18 2026c sections 8.6 and 8.7 define multipart boundaries, per-part
  headers, CRLF framing, media types, and transfer-syntax parameters. The
  parser must use the declared boundary and must not scan DICOM payload bytes
  for a guessed separator.
- PS3.18 2026c section 9.1.2 requires WADO-URI `requestType=WADO`, `studyUID`,
  `seriesUID`, and `objectUID`. F-021 requests only `application/dicom`, not a
  rendered representation.
- PS3.5 2026c section 9.1 requires a UID to contain nonempty decimal
  components separated by dots. A component has no leading zero unless it is
  exactly `0`, and the complete UID is at most 64 characters.
- PS3.18 2026c Annex F defines a QIDO JSON result as one top-level array of
  DICOM JSON data sets. Each attribute is keyed by its uppercase eight-digit
  tag, carries `vr`, and carries at most one of `Value`, `BulkDataURI`, or
  `InlineBinary`. An empty DICOM value is distinct from an absent attribute.

## What the specification does not cover

1. The HLD names `SeriesSource` but gives no Rust signature, ownership model,
   asynchronous contract, or error type.
2. The HLD does not reconcile a Rust source trait with its requirement that
   DICOMweb fetch and authentication stay in TypeScript.
3. The HLD does not define the WADO-RS or QIDO-RS resource subset, request
   options, multipart representation, DICOM JSON representation, pagination,
   cancellation, or redirect policy.
4. The HLD does not say whether QIDO-RS JSON stays in the shell or becomes the
   same lossless metadata model as Part 10 input.
5. The HLD's bulk example commits a decoded frame. It does not name a commit
   operation for a DICOMweb response or an encoded Part 10 instance.
6. The live tree has no `Session`, `alloc`, or Part 10 ingress export. F-101
   owns that command boundary even though the F-021 sprint acceptance says
   bulk bytes cross into WebAssembly once.
7. The HLD does not define authentication refresh, retry, timeout, cache, or
   CORS policy. Those are deployment concerns and stay outside the core.
8. Appendix B has no `Covered by` column and no row keyed to E3.6, despite the
   design workflow directing plans to use that column.

## Approach

1. Add a concrete TypeScript `DicomwebClient` in `@ocelli/core`. It owns URL
   construction, GET requests, `Accept` headers, HTTP status handling,
   content-type validation, abort propagation, and handing each complete
   response to the bulk writer. It accepts an injected `fetch` function so an
   application can add authentication without Ocelli storing credentials.
   Automatic retries and authentication refresh are not added.
2. Accept separate Studies Service and WADO-URI base URLs. Construct paths
   with `URL` and query components with `URLSearchParams`. Never concatenate a
   UID, search value, token, or attribute value into a URL by hand. Never place
   a request URL, query, response body, or authentication header in an error.
   Validate every study, series, instance, and WADO-URI object UID against
   PS3.5 section 9.1 before URL construction or fetch.
3. Expose QIDO-RS operations for studies, a study's series, and a study and
   series' instances. Support DICOM attribute filters plus `includefield`,
   `limit`, `offset`, and `fuzzymatching`. Each matching attribute is unique,
   UID List matching uses one comma-separated value, and only `includefield`
   repeats. Unsupported `emptyvaluematching` and `multiplevaluematching`
   controls are reserved from the generic filter path. Request
   `application/dicom+json` and identify the response to Rust as `QidoJson`.
4. Expose WADO-RS retrieval for one instance and for an explicit nonempty list
   of frames. Request original DICOM instance encoding through
   `multipart/related; type="application/dicom"; transfer-syntax=*`. Request
   frame data through the DICOM media type selected by the caller. Do not add
   whole-study or whole-series instance retrieval, because that conflicts with
   the bounded source path required for later whole-slide use.
5. Expose WADO-URI retrieval for one DICOM instance. Supply the four mandatory
   parameters and `contentType=application/dicom`. Rendered image retrieval
   and WADO-URI presentation parameters are out of scope.
6. Validate HTTP success and outer `Content-Type` in TypeScript before any
   bulk write. A successful status with a missing or incompatible media type
   is `ContentType`, not a DICOM parse error. A rejected fetch is `Network`, an
   aborted fetch is `Aborted`, and a non-success response is `HttpStatus` with
   the numeric status only. Do not read an error response body.
7. Extend `packages/core/src/bulk.ts`, which is already an allowed linear
   memory view site, with a DICOMweb response sink and writer. The order stays
   allocate, build one fresh view, set the response once, discard the view,
   commit with the explicit response kind and validated multipart metadata.
   `dicomweb.ts` never constructs a WebAssembly memory view itself.
8. Add a Rust `SeriesSource` trait and concrete `DicomwebSource` in
   `ocelli-dicom`. The trait consumes a transport-neutral `SourceResponse`
   containing bytes plus an explicit representation kind. It does not perform
   I/O, mention browser APIs, expose an async trait, or depend on
   `wasm-bindgen`. DICOMweb is the one present implementation allowed by HLD
   section 13 and the repository's structural exception.
9. Parse WADO-URI Part 10 bytes directly through F-016's `parse_part10`.
   Parse a WADO-RS DICOM multipart body using the supplied boundary, validate
   each part's headers and media type, and pass each DICOM part once to
   `parse_part10`. Parse frame multipart bodies into validated byte ranges and
   per-part media-type evidence without decoding or copying the frame payload.
10. Parse QIDO-RS DICOM JSON in Rust into F-017's public lossless metadata set
    and element types. Preserve absent versus empty, typed null slots and
    `Value` multiplicity, VR, signed numeric values, person-name objects,
    sequences, `BulkDataURI`, and `InlineBinary`. The pass-1 repair adds one
    checked F-017 null-slot representation because the original public model
    could not retain PS3.18 F.2.5 null positions for typed arrays. Sprint
    review requires duplicate JSON object members to be refused during
    deserialization, before a map can collapse equal attribute Tag keys,
    including inside sequence item data sets.
11. Return `SourceBatch` variants for query metadata, parsed Part 10
    instances, and encoded frame ranges. Keep DICOMweb request types below the
    source abstraction so future DIMSE input can produce the same Part 10
    result without changing consumers above `SeriesSource`.
12. Refuse malformed multipart framing, an absent or illegal boundary,
    incompatible part media types, malformed JSON, a non-array QIDO result,
    a repeated JSON object member, an invalid tag or VR, multiple value
    carriers, invalid base64, and any
    F-016 `ParseError`. `SourceError` stores only an error class and safe
    counts or offsets. It never stores DICOM values, UIDs, URLs, response
    bodies, headers, or upstream parser text.
13. Do not implement `Session`, a wasm export, or a concrete
    `commit_dicomweb_response` binding. F-101 owns that command boundary. F-021
    proves the shell's one-write discipline against a fake sink and proves the
    pure Rust response contract on native and wasm32. The completion claim is
    therefore a ready source path, not live end-to-end Session integration.
14. Prove sensitivity with controlled mutations. Changing one required
    WADO-URI parameter, accepting the wrong content type, splitting multipart
    content on a boundary-like payload substring, collapsing an empty JSON
    value into absence, and writing the same response twice must each make its
    named test fail before the mutation is reverted.

## Boundary and tier

- wasm-bindgen: not touched. The eventual export remains in `ocelli-wasm`
  under F-101
- Pixels across the boundary: no. The adapter permits encoded instance or
  frame response bytes to move downward through one write. Live Session
  integration is deferred to F-101. Decoded pixels never enter TypeScript
- Render-loop allocation: none. Fetch, MIME parsing, JSON parsing, and Part 10
  parsing occur in the source and decode path, outside the render loop
- unsafe: none
- Tier A (WebGPU): n/a. DICOMweb retrieval and source parsing are
  renderer-independent
- Tier B (WebGL2): n/a. The same source contract is used
- Tier C (CPU): n/a. The same source contract is used

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | URL construction covers the three QIDO resource levels, WADO-RS instance and frame resources, and the mandatory WADO-URI parameters from PS3.18 sections 9.1.2, 10.4, and 10.6 | `packages/core/src/dicomweb.test.ts` |
| unit | Empty, dot-segment, non-decimal, leading-zero, and overlength UIDs are refused before fetch, while the valid 64-character boundary remains usable, per PS3.5 section 9.1 | `packages/core/src/dicomweb.test.ts` |
| unit | Injected fetch owns authentication, AbortSignal reaches fetch, no automatic retry occurs, and HTTP, abort, network, and content-type failures remain distinct without leaking URL or response content | `packages/core/src/dicomweb.test.ts` |
| fixture | A standard-derived `application/dicom+json` array preserves absence, empty values, multiplicity, VR, signed numbers, PN components, SQ items, `BulkDataURI`, and `InlineBinary` in F-017 metadata types, per PS3.18 Annex F | `crates/ocelli-dicom/tests/dicomweb.rs` |
| fixture | Duplicate attribute Tag keys are refused before map materialization at the root data set and inside an SQ item, per PS3.18 sections F.2.2 and F.3.1 | `crates/ocelli-dicom/tests/dicomweb.rs` |
| fixture | Single and multiple `multipart/related` DICOM parts are split only at legal CRLF-framed boundaries, validate per-part media types, and parse through F-016, per PS3.18 sections 8.6 and 10.4 | `crates/ocelli-dicom/tests/dicomweb.rs` |
| fixture | Frame multipart input returns ordered zero-copy byte ranges and media-type evidence, including a payload containing boundary-like bytes that are not delimiter lines | `crates/ocelli-dicom/tests/dicomweb.rs` |
| unit | Every malformed response class is refused without panic and `SourceError` debug or display output contains no DICOM value, UID, URL, header, or payload | `crates/ocelli-dicom/tests/dicomweb.rs` |
| cross-target | The same `SeriesSource` and response parser compile for native and wasm32 with no `wasm-bindgen` dependency | `bin/ocelli.sh check ocelli-dicom` and `bin/ocelli.sh native` |
| boundary | A fake sink that grows memory during allocation observes exactly one fresh typed-array write and one commit per HTTP response | `packages/core/src/bulk.test.ts` and `packages/core/src/dicomweb.test.ts` |
| mutation | Required request fields, content-type checks, multipart delimiter anchoring, empty-value preservation, and one-write ownership each have a controlled red mutation | recorded in the clean feature review |

No pixel or geometry arithmetic is introduced, so a pixel fixture under HLD
27.2 R3 is not applicable.

## Parity surface covered

Appendix B has no DICOMweb row, no `Covered by` column, and no E3.6 mapping.
F-021 enables the source path used by DICOM ingest without changing an
Appendix B count. The tracked parity appendix remains unchanged.

## Deviations

Existing D-02 applies the `ocelli-dicom` crate name. Existing D-18 applies the
dicom-rs component dependency shape and the crate's `std` posture. No new HLD
deviation is required.

## LLD impact

- Create `docs/lld/dicomweb.md` for endpoint scope, source response kinds,
  content negotiation, error ownership, multipart handling, DICOM JSON mapping,
  and the deferred Session seam.
- Update `docs/lld/README.md` to index `dicomweb.md` and list F-021.
- Update `docs/lld/dicom-ingest.md` with `SeriesSource`, the DICOMweb response
  path, and its use of `parse_part10`.
- Update `docs/lld/build-targets.md` only if the added Rust JSON dependency
  changes the measured native or wasm dependency account.

## Shared file ownership and write set

F-017 lands before F-021 in the implementation wave because QIDO parsing uses
its public metadata types. The approved pass-1 remediation reopens the F-017
seam only for `NullSlots`, `MetadataValue::WithNullSlots`, the checked
`MetadataElement::with_null_slots` constructor, and their independent metadata
fixture. No generic unchecked value constructor is added.

F-021 exclusively owns these created files:

- `crates/ocelli-dicom/src/dicomweb.rs`
- `crates/ocelli-dicom/tests/dicomweb.rs`
- `packages/core/src/dicomweb.ts`
- `packages/core/src/dicomweb.test.ts`
- `docs/lld/dicomweb.md`

F-021 modifies these implementation files:

- `Cargo.toml`
- `Cargo.lock`
- `crates/ocelli-dicom/Cargo.toml`
- `crates/ocelli-dicom/src/lib.rs`
- `crates/ocelli-dicom/src/metadata.rs`, approved null-slot seam repair only
- `crates/ocelli-dicom/tests/metadata.rs`, independent seam fixture only
- `packages/core/src/bulk.ts`
- `packages/core/src/bulk.test.ts`
- `packages/core/src/index.ts`
- `packages/core/src/index.test.ts`

The approved plan originally listed the workspace `Cargo.toml` for a new
`serde_json` declaration, but that dependency already existed in the workspace
from F-011. Pass-2 remediation adds `uriparse` at the workspace and member
levels because Content-Location permits relative URI references and an
absolute-only URL parser would reject valid PS3.18 input. The root manifest
therefore returns to the write set for this discovered dependency edge.
Sprint-review remediation adds the existing workspace `serde` dependency at
the member level for the duplicate-detecting deserialization visitor.

The following are shared sprint files. The serial integrator or the
`/complete-feature` workflow owns their final edit after concurrent feature
work is integrated:

- `.claude/plans/F-021-design.md`
- `docs/lld/README.md`
- `docs/lld/dicom-ingest.md`
- `docs/lld/build-targets.md`, only if dependency evidence changes
- `docs/sprints/CURRENT_SPRINT.md`
- `docs/sprints/BACKLOG.md`
- `docs/sprints/SPRINT_TRACKER.md`
- `docs/sprints/AS_BUILT.md`
- `CHANGELOG.md`

Likely collision points are `Cargo.toml`, `Cargo.lock`,
`crates/ocelli-dicom/Cargo.toml`, `crates/ocelli-dicom/src/lib.rs`, and
`docs/lld/README.md`. F-017 and F-022 also touch `ocelli-dicom`. These files
must be integrated by a three-way merge or handled serially, never overwritten
from a stale worktree.

F-021 does not edit `crates/ocelli-wasm`, `ci/error-codes.json`,
`packages/core/src/errors.ts`, or `docs/hld/DEVIATIONS.md`. Its only edits to
F-017-owned metadata implementation are the scoped `NullSlots` seam in
`metadata.rs` and its independent metadata fixture, as authorized above.

## Open questions

None.

## Design-round decisions

1. QIDO-RS DICOM JSON is parsed in Rust into F-017's public lossless metadata
   element and set types. Query metadata and Part 10 metadata therefore retain
   the same absence, empty, multiplicity, VR, and signedness semantics.
2. F-017 owns the metadata implementation. F-021 does not create a parallel
   metadata model. The approved pass-1 seam repair adds only the checked typed
   null-slot representation needed to preserve PS3.18 F.2.5 multiplicity.
3. TypeScript owns fetch, authentication injection, HTTP status, content-type
   validation, and cancellation. It passes response bytes plus an explicit
   response kind to the Rust-facing source contract.
4. F-021 implements the pure Rust `SeriesSource` and DICOMweb response contract
   plus the TypeScript fetch adapter at the existing bulk-write seam. F-101
   owns the eventual live `Session` command and commit binding, so F-021 does
   not claim that integration.
