# F-021 review, pass 3

**Reviewed**: `f54b842364ed731fa465c91688dc9167c557a117` through the exact staged 23-file working tree
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-2 D1 is fixed. The generic QIDO filter path reserves
  `emptyvaluematching` and `multiplevaluematching` alongside the four exposed
  controls and refuses every reserved name before fetch. The public
  `QidoSearchOptions` remains bounded to `includefield`, `limit`, `offset`, and
  `fuzzymatching`, so the repair adds no typed API scope.
- Removing the two newly reserved names in a disposable exact-index copy made
  the named QIDO refusal test fail because the request resolved and fetched.
- Pass-2 D2 is fixed. The plan and LLD now say each matching attribute is
  unique, UID List matching is one comma-separated value, and only
  `includefield` repeats. They describe the two unsupported current controls as
  reserved rather than implemented. This agrees with DICOM PS3.18 2026c
  section 8.3.4 and the staged public API.
- Pass-2 D3 is fixed. Required multipart header names are compared without
  trimming the field name, while ordinary whitespace after the colon is
  removed from the value. Restoring field-name trimming in a disposable copy
  made the exact-header test fail by accepting `Content-Type :`.
- Pass-2 D4 is fixed. Content-Location is checked with
  `uriparse::URIReference`, which implements RFC 3986 URI-reference grammar.
  The staged fixture rejects internal spaces and malformed percent escapes and
  accepts a legal relative reference containing path traversal and a query.
- Removing URI-reference validation in a disposable copy made the invalid
  Content-Location fixture fail. Adding an absolute-only condition made the
  legal relative-reference fixture fail. A separate disposable positive probe
  accepted both an absolute HTTPS URI and a network-path reference.
- Content-Location spelling is discarded after validation. `PartHeaders`,
  `MultipartPart`, and every `SourceError` variant contain no location string.
  A disposable invalid location containing a synthetic marker returned
  `InvalidMultipart`, and neither its Display nor Debug representation retained
  the marker.
- `uriparse` 0.6.4 is declared once in workspace dependencies and directly,
  non-optionally by `ocelli-dicom`. Cargo metadata resolves it from the crates.io
  registry with an MIT license and the lockfile records the registry checksum.
  `bin/ocelli.sh gate provenance` passed over 572 files.
- `bin/ocelli.sh native` passed all four steps. It built the shared Rust crates,
  including `ocelli-dicom`, for native and `wasm32-unknown-unknown` and reported
  all 14 direct dependency feature selections identical across targets. The
  build LLD accurately records `uriparse` as shared Rust with no browser binding
  and no no-std claim.
- The complete `ocelli-dicom` suite passed 76 tests with the one declared corpus
  test ignored. This includes 11 DICOMweb fixtures, 18 metadata fixtures, 41
  Part 10 fixtures, and 6 crate unit tests.
- The focused TypeScript suite passed all 23 tests across `dicomweb.test.ts`,
  `bulk.test.ts`, and `index.test.ts`.
- `bin/ocelli.sh check ocelli-dicom`, `bin/ocelli.sh clippy ocelli-dicom`,
  `bin/ocelli.sh fmt --check`, `npm run typecheck`, and `npm run lint` passed.
- `python3 scripts/prose_check.py` passed over 233 files.
- `bin/ocelli.sh gate content` and `git diff --cached --check` passed before
  this review artifact was added.
- Pass-1 D1 through D11 remain repaired. Unsigned QIDO pagination, unique
  matching attributes, strictly ascending unique frames, fetch and body-read
  failure classification, safe invalid URLs, pre-write boundary validation,
  parameterized media-type retention, mandatory multipart resource headers,
  SV and UV string inputs, typed null positions, and Group Length refusal all
  remain covered by focused positive and negative tests.
- Multipart resource parsing still requires ordered Content-Type,
  Content-Location, and exactly one Content-Length or Transfer-Encoding header.
  Content-Length uses exact ASCII decimal syntax and must equal the payload.
  Frame parts retain their complete parameterized media type and observable
  transfer syntax without copying payloads out of the owned response.
- DICOM JSON still preserves typed null positions, including PN and AT, all-null
  multiplicity, and empty-object SQ item semantics through F-017's checked
  NullSlots seam. Group Length attributes remain refused recursively.
- Fetch authentication stays caller-owned, cancellation reaches fetch, errors
  retain no URL, UID, header, query, credential, response body, parser text, or
  Content-Location, and failed requests make no bulk commit.
- The bulk boundary still performs one allocation, one fresh linear-memory
  view, one copy, and one commit. F-101 remains the truthful explicit deferral
  for the live Session command and wasm commit binding.
- The complete staged diff adds no `unsafe`, no `wasm-bindgen` outside
  `ocelli-wasm`, no pixel arithmetic, no render-loop work, no second source
  implementation, and no undeclared public API.
